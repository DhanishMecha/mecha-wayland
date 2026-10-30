use app::prelude::*;
use dbus::fake::FakeBus;
use dbus::*;

#[derive(Default)]
struct Log {
    owners: Vec<(String, Option<String>, Option<String>)>,
    signals: Vec<SignalReceived>,
}
impl Resource for Log {}

fn on_owner(app: &mut App, e: &NameOwnerChanged) {
    let name = |n: &Option<BusName>| n.as_ref().map(|n| n.as_str().to_owned());
    app.resource_mut::<Log>()
        .owners
        .push((e.name.as_str().to_owned(), name(&e.old), name(&e.new)));
}
fn on_signal(app: &mut App, e: &SignalReceived) {
    app.resource_mut::<Log>().signals.push(e.clone());
}

fn fake() -> FakeBus {
    let mut f = FakeBus::new();
    f.app
        .init_resource::<Log>()
        .system(on_owner)
        .system(on_signal);
    f.received();
    f
}

fn owners(f: &mut FakeBus) -> Vec<(String, Option<String>, Option<String>)> {
    std::mem::take(&mut f.app.resource_mut::<Log>().owners)
}

fn change(name: &str, old: &str, new: &str) -> Message {
    Message::signal(
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
        "NameOwnerChanged",
    )
    .with_sender("org.freedesktop.DBus")
    .with_body(&(name, old, new))
}

fn watch(f: &mut FakeBus, name: &str) {
    f.app
        .resource_mut::<DBus>()
        .session()
        .watch_name(BusName::new(name).unwrap());
}

fn unwatch(f: &mut FakeBus, name: &str) {
    f.app
        .resource_mut::<DBus>()
        .session()
        .unwatch_name(BusName::new(name).unwrap());
}

fn count(f: &mut FakeBus, member: &str) -> usize {
    f.received()
        .iter()
        .filter(|m| m.member().is_some_and(|x| x.as_str() == member))
        .count()
}

fn some(s: &str) -> Option<String> {
    Some(s.to_owned())
}

#[test]
fn the_owner_now_then_each_change() {
    let mut f = fake();
    f.owners.insert("org.bluez".into(), ":1.5".into());
    watch(&mut f, "org.bluez");
    f.turn();
    f.send(change("org.bluez", ":1.5", ""));
    f.send(change("org.other", "", ":1.8"));
    f.send(change("org.bluez", "", ":1.7"));
    f.turn();
    assert_eq!(
        owners(&mut f),
        [
            ("org.bluez".into(), None, some(":1.5")),
            ("org.bluez".into(), some(":1.5"), None),
            ("org.bluez".into(), None, some(":1.7")),
        ]
    );
    assert!(
        f.app.resource::<Log>().signals.is_empty(),
        "the internal rule raises no SignalReceived"
    );
}

#[test]
fn an_unowned_name_says_nobody_owns_it() {
    let mut f = fake();
    watch(&mut f, "org.bluez");
    f.turn();
    assert_eq!(owners(&mut f), [("org.bluez".into(), None, None)]);
}

#[test]
fn a_change_before_the_first_answer_is_not_raised_twice() {
    let mut f = fake();
    f.auto_daemon = false;
    watch(&mut f, "org.bluez");
    f.turn();
    let lookup = f.expect_call("GetNameOwner");
    // The daemon reports a change, then answers with the owner after it.
    f.send(change("org.bluez", "", ":1.5"));
    f.reply(&lookup, &(":1.5",));
    f.turn();
    assert_eq!(owners(&mut f), [("org.bluez".into(), None, some(":1.5"))]);
}

#[test]
fn watches_count_and_share_one_rule() {
    let mut f = fake();
    f.owners.insert("org.bluez".into(), ":1.5".into());
    watch(&mut f, "org.bluez");
    f.turn();
    watch(&mut f, "org.bluez");
    f.turn();
    assert_eq!(count(&mut f, "AddMatch"), 1);
    assert_eq!(owners(&mut f).len(), 2, "each watch gets its first answer");

    unwatch(&mut f, "org.bluez");
    f.turn();
    assert_eq!(count(&mut f, "RemoveMatch"), 0, "one watch is left");
    f.send(change("org.bluez", ":1.5", ""));
    f.turn();
    assert_eq!(owners(&mut f).len(), 1);

    unwatch(&mut f, "org.bluez");
    f.turn();
    assert_eq!(count(&mut f, "RemoveMatch"), 1);
    f.send(change("org.bluez", "", ":1.7"));
    f.turn();
    assert!(owners(&mut f).is_empty(), "no watch is left");
}

#[test]
fn unwatching_keeps_a_rule_that_follows_the_same_name() {
    let mut f = fake();
    f.owners.insert("org.bluez".into(), ":1.5".into());
    let id = f
        .app
        .resource_mut::<DBus>()
        .session()
        .add_match(MatchRule::signal().sender("org.bluez").member("M"));
    watch(&mut f, "org.bluez");
    unwatch(&mut f, "org.bluez");
    f.turn();
    f.send(change("org.bluez", ":1.5", ":1.7"));
    f.send(
        Message::signal("/", "a.b", "M")
            .with_sender(":1.7")
            .with_body(&()),
    );
    f.turn();
    let log = f.app.resource::<Log>();
    assert_eq!(log.signals.len(), 1, "the rule still follows the owner");
    assert_eq!(log.signals[0].matches.as_slice(), [id]);
}

#[test]
fn a_hang_up_ends_every_watch() {
    let mut f = fake();
    watch(&mut f, "org.bluez");
    f.turn();
    owners(&mut f);
    let mut app = f.hang_up();
    ring::Ring::turn(&mut app);
    assert!(app.resource::<Log>().owners.is_empty());
}

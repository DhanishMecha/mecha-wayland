use app::prelude::*;
use dbus::fake::FakeBus;
use dbus::*;

/// The caller's own signal a rule's signals are mapped to.
#[derive(Debug, Clone, PartialEq)]
struct Changed(String);
impl Signal for Changed {}

#[derive(Default)]
struct Log {
    changed: Vec<String>,
    signals: Vec<SignalReceived>,
}
impl Resource for Log {}

fn on_changed(app: &mut App, e: &Changed) {
    app.resource_mut::<Log>().changed.push(e.0.clone());
}
fn on_signal(app: &mut App, e: &SignalReceived) {
    app.resource_mut::<Log>().signals.push(e.clone());
}

fn fake() -> FakeBus {
    let mut f = FakeBus::new();
    f.app
        .init_resource::<Log>()
        .system(on_changed)
        .system(on_signal);
    f.received();
    f
}

/// `Changed(s)` for a body of one string; `None` for anything else.
fn changed(msg: &Message) -> Option<Changed> {
    msg.body::<(String,)>().ok().map(|(s,)| Changed(s))
}

fn signal(iface: &str, member: &str) -> Message {
    Message::signal(
        "/p",
        InterfaceName::new(iface).unwrap(),
        MemberName::new(member).unwrap(),
    )
    .with_sender(":1.9")
}

fn take(f: &mut FakeBus) -> (Vec<String>, Vec<SignalReceived>) {
    let mut log = f.app.resource_mut::<Log>();
    (
        std::mem::take(&mut log.changed),
        std::mem::take(&mut log.signals),
    )
}

#[test]
fn a_mapped_rule_raises_the_callers_signal_instead() {
    let mut f = fake();
    f.app
        .resource_mut::<DBus>()
        .session()
        .add_match_as(MatchRule::signal().member("Changed"), changed);
    f.turn();
    f.send(signal("a.b", "Changed").with_body(&("on",)));
    f.turn();
    let (changed, signals) = take(&mut f);
    assert_eq!(changed, ["on"]);
    assert!(signals.is_empty(), "no SignalReceived for a mapped rule");
}

#[test]
fn none_drops_the_signal() {
    let mut f = fake();
    f.app
        .resource_mut::<DBus>()
        .session()
        .add_match_as(MatchRule::signal().member("Changed"), changed);
    f.turn();
    f.send(signal("a.b", "Changed").with_body(&(7u32,)));
    f.turn();
    let (changed, signals) = take(&mut f);
    assert!(changed.is_empty());
    assert!(signals.is_empty());
}

#[test]
fn unmapped_rules_matching_too_still_get_signal_received() {
    let mut f = fake();
    let (mapped, plain) = {
        let mut dbus = f.app.resource_mut::<DBus>();
        let mut bus = dbus.session();
        let mapped = bus.add_match_as(MatchRule::signal().member("Changed"), changed);
        let plain = bus.add_match(MatchRule::signal().interface("a.b"));
        (mapped, plain)
    };
    f.turn();
    f.send(signal("a.b", "Changed").with_body(&("both",)));
    f.send(signal("x.y", "Changed").with_body(&("mapped only",)));
    f.turn();
    let (changed, signals) = take(&mut f);
    assert_eq!(changed, ["both", "mapped only"]);
    assert_eq!(signals.len(), 1, "only the signal the plain rule matches");
    assert_eq!(signals[0].matches.as_slice(), [plain]);
    assert!(!signals[0].matches.contains(&mapped));
}

#[test]
fn identical_rules_one_mapped_share_one_add_match() {
    let mut f = fake();
    let rule = MatchRule::signal().member("Changed");
    let plain = {
        let mut dbus = f.app.resource_mut::<DBus>();
        let mut bus = dbus.session();
        bus.add_match_as(rule.clone(), changed);
        bus.add_match(rule)
    };
    f.turn();
    let adds = f
        .received()
        .iter()
        .filter(|m| m.member().is_some_and(|x| x.as_str() == "AddMatch"))
        .count();
    assert_eq!(adds, 1);
    f.send(signal("a.b", "Changed").with_body(&("x",)));
    f.turn();
    let (changed, signals) = take(&mut f);
    assert_eq!(changed, ["x"]);
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].matches.as_slice(), [plain]);
}

#[test]
fn mapping_an_existing_id_and_mapping_again_replaces() {
    let mut f = fake();
    let id = f
        .app
        .resource_mut::<DBus>()
        .session()
        .add_match(MatchRule::signal().member("Changed"));
    f.app
        .resource_mut::<DBus>()
        .session()
        .map_signals(id, |_| Some(Changed("first".into())));
    f.app
        .resource_mut::<DBus>()
        .session()
        .map_signals(id, |_| Some(Changed("second".into())));
    f.turn();
    f.send(signal("a.b", "Changed"));
    f.turn();
    let (changed, signals) = take(&mut f);
    assert_eq!(changed, ["second"]);
    assert!(signals.is_empty());
}

#[test]
fn a_removed_rule_forgets_its_map_and_cannot_be_mapped() {
    let mut f = fake();
    let id = f
        .app
        .resource_mut::<DBus>()
        .session()
        .add_match_as(MatchRule::signal().member("Changed"), changed);
    // A plain rule keeps the daemon sending these signals.
    f.app
        .resource_mut::<DBus>()
        .session()
        .add_match(MatchRule::signal().interface("a.b"));
    f.app.resource_mut::<DBus>().session().remove_match(id);
    f.app
        .resource_mut::<DBus>()
        .session()
        .map_signals(id, |_| Some(Changed("late".into())));
    f.turn();
    f.send(signal("a.b", "Changed").with_body(&("x",)));
    f.turn();
    let (changed, signals) = take(&mut f);
    assert!(changed.is_empty());
    assert_eq!(signals.len(), 1);
    assert!(!signals[0].matches.contains(&id));
}

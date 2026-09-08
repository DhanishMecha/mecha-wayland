//! Systems, handlers, the two queues, and the take-out edge cases.

use std::cell::RefCell;

use app::prelude::*;

// ── fixtures ─────────────────────────────────────────────────────────────────

// Systems can't capture, so they log through a thread-local.
thread_local! {
    static LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn log(s: impl Into<String>) {
    LOG.with(|l| l.borrow_mut().push(s.into()));
}

fn take_log() -> Vec<String> {
    LOG.with(|l| std::mem::take(&mut *l.borrow_mut()))
}

struct Counter(u32);
impl Widget for Counter {}

struct Label(String);
impl Widget for Label {}

struct Inc;
impl Event for Inc {}

struct Add(u32);
impl Event for Add {}

struct Rename(&'static str);
impl Event for Rename {}

struct Ping;
impl Signal for Ping {}

struct Pong;
impl Signal for Pong {}

struct CounterBuilder;
impl WidgetBuild for CounterBuilder {
    type Widget = Counter;
    fn spawn(self, me: Handle<Counter>, s: &mut Spawner<Counter>) -> Counter {
        s.on(me, |ctx: &mut Context<Counter>, _: &Inc| ctx.me().0 += 1);
        s.on(me, |ctx: &mut Context<Counter>, e: &Add| ctx.me().0 += e.0);
        Counter(0)
    }
}

struct LabelBuilder(&'static str);
impl WidgetBuild for LabelBuilder {
    type Widget = Label;
    fn spawn(self, me: Handle<Label>, s: &mut Spawner<Label>) -> Label {
        s.on(me, |ctx: &mut Context<Label>, e: &Rename| {
            ctx.me().0 = e.0.into()
        });
        Label(self.0.into())
    }
}

fn count(app: &App, h: Handle<Counter>) -> u32 {
    app.widget::<Counter>(h).unwrap().0
}

// ── systems ──────────────────────────────────────────────────────────────────

#[test]
fn systems_run_in_registration_order_for_their_signal_only() {
    fn a(_: &mut App, _: &Ping) {
        log("a");
    }
    fn b(_: &mut App, _: &Ping) {
        log("b");
    }
    fn other(_: &mut App, _: &Pong) {
        log("pong");
    }

    let mut app = App::new();
    app.system(b).system(a).system(other);
    app.signal(Ping);
    assert_eq!(
        take_log(),
        Vec::<String>::new(),
        "nothing runs before flush"
    );
    app.flush();
    assert_eq!(take_log(), ["b", "a"]);
}

#[test]
fn same_system_twice_runs_twice() {
    fn a(_: &mut App, _: &Ping) {
        log("a");
    }
    let mut app = App::new();
    app.system(a).system(a);
    app.signal(Ping);
    app.flush();
    assert_eq!(take_log(), ["a", "a"]);
}

#[test]
fn signal_with_no_systems_is_dropped() {
    let mut app = App::new();
    app.signal(Ping);
    app.flush();
    assert_eq!(take_log(), Vec::<String>::new());
}

#[test]
fn a_system_can_spawn_remove_and_signal() {
    fn on_ping(app: &mut App, _: &Ping) {
        let c = app.spawn(app.root(), CounterBuilder).unwrap();
        app.emit(Add(5), &[c.id()]);
        app.signal(Pong);
    }
    fn on_pong(app: &mut App, _: &Pong) {
        let ids: Vec<_> = app.widgets::<Counter>().map(|(id, c)| (id, c.0)).collect();
        log(format!("pong {ids:?}"));
        for (id, _) in ids {
            app.remove(id).unwrap();
        }
    }
    let mut app = App::new();
    app.system(on_ping).system(on_pong);
    app.signal(Ping);
    app.flush();
    // The Add ran before Pong (events first), so Pong saw 5.
    let l = take_log();
    assert_eq!(l.len(), 1);
    assert!(l[0].ends_with(", 5)]"), "{}", l[0]);
    assert_eq!(app.widgets::<Counter>().count(), 0);
}

#[test]
fn system_registered_mid_pass_runs_in_that_pass() {
    fn first(app: &mut App, _: &Ping) {
        log("first");
        app.system(second);
    }
    fn second(_: &mut App, _: &Ping) {
        log("second");
    }
    let mut app = App::new();
    app.system(first);
    app.signal(Ping);
    app.flush();
    assert_eq!(take_log(), ["first", "second"]);
}

// ── handlers ─────────────────────────────────────────────────────────────────

#[test]
fn emit_reaches_only_the_named_nodes() {
    let mut app = App::new();
    let a = app.spawn(app.root(), CounterBuilder).unwrap();
    let b = app.spawn(app.root(), CounterBuilder).unwrap();
    let c = app.spawn(a, CounterBuilder).unwrap();

    app.emit(Inc, &[a.id(), c.id()]);
    assert_eq!(count(&app, a), 0, "queued, not run");
    app.flush();
    assert_eq!((count(&app, a), count(&app, b), count(&app, c)), (1, 0, 1));
}

#[test]
fn emit_to_same_node_twice_runs_twice() {
    let mut app = App::new();
    let a = app.spawn(app.root(), CounterBuilder).unwrap();
    app.emit(Inc, &[a.id(), a.id()]);
    app.flush();
    assert_eq!(count(&app, a), 2);
}

#[test]
fn emit_all_reaches_every_node_with_a_handler() {
    let mut app = App::new();
    let a = app.spawn(app.root(), CounterBuilder).unwrap();
    let l = app.spawn(app.root(), LabelBuilder("x")).unwrap();
    let b = app.spawn(l, CounterBuilder).unwrap();

    app.emit_all(Inc);
    app.emit_all(Rename("y"));
    app.flush();
    assert_eq!((count(&app, a), count(&app, b)), (1, 1));
    assert_eq!(app.widget::<Label>(l).unwrap().0, "y");
}

#[test]
fn emit_all_snapshots_targets_at_emit_time() {
    let mut app = App::new();
    let a = app.spawn(app.root(), CounterBuilder).unwrap();
    app.emit_all(Inc);
    let b = app.spawn(app.root(), CounterBuilder).unwrap();
    app.flush();
    assert_eq!((count(&app, a), count(&app, b)), (1, 0));
}

#[test]
fn handlers_for_one_event_run_in_registration_order() {
    struct Order;
    impl Widget for Order {}
    struct Step;
    impl Event for Step {}
    struct OrderBuilder;
    impl WidgetBuild for OrderBuilder {
        type Widget = Order;
        fn spawn(self, me: Handle<Order>, s: &mut Spawner<Order>) -> Order {
            s.on(me, |_: &mut Context<Order>, _: &Step| log("1"));
            s.on(me, |_: &mut Context<Order>, _: &Step| log("2"));
            s.on(me, |_: &mut Context<Order>, _: &Step| log("3"));
            Order
        }
    }
    let mut app = App::new();
    let o = app.spawn(app.root(), OrderBuilder).unwrap();
    app.emit(Step, &[o.id()]);
    app.flush();
    assert_eq!(take_log(), ["1", "2", "3"]);
}

#[test]
fn unhandled_root_and_stale_targets_are_skipped() {
    let mut app = App::new();
    let a = app.spawn(app.root(), CounterBuilder).unwrap();
    let l = app.spawn(app.root(), LabelBuilder("x")).unwrap();
    let gone = app.spawn(app.root(), CounterBuilder).unwrap();
    app.remove(gone).unwrap();

    // Label has no Inc handler; root has none; gone is stale.
    app.emit(Inc, &[l.id(), app.root(), gone.id(), a.id()]);
    app.flush();
    assert_eq!(count(&app, a), 1);
}

#[test]
fn target_removed_after_emit_is_skipped_at_flush() {
    let mut app = App::new();
    let a = app.spawn(app.root(), CounterBuilder).unwrap();
    let b = app.spawn(app.root(), CounterBuilder).unwrap();
    app.emit(Inc, &[a.id(), b.id()]);
    app.remove(a).unwrap();
    app.flush();
    assert_eq!(count(&app, b), 1);
    assert!(app.widget::<Counter>(a).is_none());
}

#[test]
fn on_with_a_stale_handle_fails_the_build() {
    struct Bad(Handle<Counter>);
    impl WidgetBuild for Bad {
        type Widget = Counter;
        fn spawn(self, _me: Handle<Counter>, s: &mut Spawner<Counter>) -> Counter {
            s.on(self.0, |_: &mut Context<Counter>, _: &Inc| {});
            Counter(0)
        }
    }
    let mut app = App::new();
    let gone = app.spawn(app.root(), CounterBuilder).unwrap();
    app.remove(gone).unwrap();
    assert!(matches!(
        app.spawn(app.root(), Bad(gone)),
        Err(app::Error::Stale)
    ));
    assert_eq!(app.children(app.root()).unwrap().len(), 0);
}

// ── context ──────────────────────────────────────────────────────────────────

#[test]
fn handler_can_emit_and_signal_and_they_are_queued() {
    struct Relay;
    impl Widget for Relay {}
    struct Kick;
    impl Event for Kick {}
    struct RelayBuilder;
    impl WidgetBuild for RelayBuilder {
        type Widget = Relay;
        fn spawn(self, me: Handle<Relay>, s: &mut Spawner<Relay>) -> Relay {
            s.on(me, |ctx: &mut Context<Relay>, _: &Kick| {
                ctx.emit_all(Inc);
                ctx.signal(Ping);
                log("kick");
            });
            Relay
        }
    }
    fn on_ping(app: &mut App, _: &Ping) {
        let total: u32 = app.widgets::<Counter>().map(|(_, c)| c.0).sum();
        log(format!("ping {total}"));
    }

    let mut app = App::new();
    app.system(on_ping);
    app.spawn(app.root(), CounterBuilder).unwrap();
    let r = app.spawn(app.root(), RelayBuilder).unwrap();
    app.emit(Kick, &[r.id()]);
    app.flush();
    // Handler ran, then its Inc (event) before its Ping (signal).
    assert_eq!(take_log(), ["kick", "ping 1"]);
}

#[test]
fn at_reads_another_widget_and_none_for_self() {
    struct Reader;
    impl Widget for Reader {}
    struct Look(Handle<Counter>, Handle<Reader>);
    impl Event for Look {}
    struct ReaderBuilder;
    impl WidgetBuild for ReaderBuilder {
        type Widget = Reader;
        fn spawn(self, me: Handle<Reader>, s: &mut Spawner<Reader>) -> Reader {
            s.on(me, |ctx: &mut Context<Reader>, e: &Look| {
                let other = ctx.at(e.0).map(|c| c.0);
                let myself = ctx.at(e.1).is_some();
                log(format!("other={other:?} self={myself}"));
                if let Some(c) = ctx.at(e.0) {
                    c.0 = 99;
                }
            });
            Reader
        }
    }
    let mut app = App::new();
    let c = app.spawn(app.root(), CounterBuilder).unwrap();
    let r = app.spawn(app.root(), ReaderBuilder).unwrap();
    app.emit(Look(c, r), &[r.id()]);
    app.flush();
    assert_eq!(take_log(), ["other=Some(0) self=false"]);
    assert_eq!(count(&app, c), 99);
    // The reader is back in its slot afterwards.
    assert!(app.widget::<Reader>(r).is_some());
}

#[test]
fn widget_is_absent_from_lookups_only_while_its_handler_runs() {
    struct Probe;
    impl Widget for Probe {}
    struct Check;
    impl Event for Check {}
    struct ProbeBuilder;
    impl WidgetBuild for ProbeBuilder {
        type Widget = Probe;
        fn spawn(self, me: Handle<Probe>, s: &mut Spawner<Probe>) -> Probe {
            s.on(me, |ctx: &mut Context<Probe>, _: &Check| {
                // No public way to reach the app here; `at` on self is the
                // observable form of "taken out".
                assert!(ctx.at(ctx.handle()).is_none());
            });
            Probe
        }
    }
    let mut app = App::new();
    let p = app.spawn(app.root(), ProbeBuilder).unwrap();
    assert!(app.widget::<Probe>(p).is_some());
    app.emit(Check, &[p.id()]);
    app.flush();
    assert!(app.widget::<Probe>(p).is_some());
    assert_eq!(app.widgets::<Probe>().count(), 1);
}

#[test]
fn panicking_handler_leaves_widget_and_handlers_in_place() {
    struct Fuse(u32);
    impl Widget for Fuse {}
    struct Blow;
    impl Event for Blow {}
    struct FuseBuilder;
    impl WidgetBuild for FuseBuilder {
        type Widget = Fuse;
        fn spawn(self, me: Handle<Fuse>, s: &mut Spawner<Fuse>) -> Fuse {
            s.on(me, |ctx: &mut Context<Fuse>, _: &Blow| {
                ctx.me().0 += 1;
                if ctx.me().0 == 1 {
                    panic!("blown");
                }
            });
            // A second handler that never gets to run on the first pass.
            s.on(me, |_: &mut Context<Fuse>, _: &Blow| log("second"));
            Fuse(0)
        }
    }

    let mut app = App::new();
    let f = app.spawn(app.root(), FuseBuilder).unwrap();
    app.emit(Blow, &[f.id()]);

    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| app.flush()));
    std::panic::set_hook(hook);
    assert!(
        result.is_err(),
        "the handler's panic propagates out of flush"
    );

    // The widget went back on unwind, with the mutation it made before
    // panicking, and the handler list survived too.
    assert_eq!(app.widget::<Fuse>(f).unwrap().0, 1);
    app.emit(Blow, &[f.id()]);
    app.flush();
    assert_eq!(app.widget::<Fuse>(f).unwrap().0, 2);
    assert_eq!(take_log(), ["second"]);
}

// ── flush ordering ───────────────────────────────────────────────────────────

#[test]
fn flush_runs_pending_events_before_each_signal() {
    fn on_ping(app: &mut App, _: &Ping) {
        let ids: Vec<_> = app.widgets::<Counter>().map(|(id, _)| id).collect();
        log(format!(
            "ping sees {}",
            app.widget::<Counter>(ids[0]).unwrap().0
        ));
        app.emit(Inc, &ids);
        app.signal(Pong);
    }
    fn on_pong(app: &mut App, _: &Pong) {
        let v = app.widgets::<Counter>().map(|(_, c)| c.0).next().unwrap();
        log(format!("pong sees {v}"));
    }
    let mut app = App::new();
    app.system(on_ping).system(on_pong);
    let c = app.spawn(app.root(), CounterBuilder).unwrap();

    // Queue order: Ping, Inc. Events first, so Ping sees the Inc applied.
    app.signal(Ping);
    app.emit(Inc, &[c.id()]);
    app.flush();
    assert_eq!(take_log(), ["ping sees 1", "pong sees 2"]);
}

#[test]
fn flush_on_empty_queues_is_a_noop() {
    let mut app = App::new();
    app.flush();
    app.flush();
}

// ── runner ───────────────────────────────────────────────────────────────────

#[test]
fn run_hands_the_app_to_the_runner() {
    fn once(mut app: App) {
        app.signal(Ping);
        app.flush();
        log(format!(
            "ran with {} counters",
            app.widgets::<Counter>().count()
        ));
    }
    fn on_ping(_: &mut App, _: &Ping) {
        log("ping");
    }
    let mut app = App::new();
    app.system(on_ping).set_runner(once);
    app.spawn(app.root(), CounterBuilder).unwrap();
    app.run();
    assert_eq!(take_log(), ["ping", "ran with 1 counters"]);
}

#[test]
fn tick_is_a_signal_systems_can_hook() {
    // The default runner loops forever, so mirror one iteration of its body
    // in a runner that returns.
    fn on_tick(_: &mut App, _: &Tick) {
        log("tick");
    }
    fn one_iteration(mut app: App) {
        app.signal(Tick);
        app.flush();
    }
    let mut app = App::new();
    app.system(on_tick).set_runner(one_iteration);
    app.run();
    assert_eq!(take_log(), ["tick"]);
}

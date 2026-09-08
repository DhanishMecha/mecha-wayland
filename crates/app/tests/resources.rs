use app::prelude::*;

// ── fixtures ─────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
struct Clock(u64);
impl Resource for Clock {}

struct Theme(&'static str);
impl Resource for Theme {}

struct Counter(u32);
impl Widget for Counter {}

struct Bump;
impl Event for Bump {}

struct Sync;
impl Signal for Sync {}

/// Every handler run adds to the counter *and* stamps the clock.
struct CounterBuilder;
impl WidgetBuild for CounterBuilder {
    type Widget = Counter;
    fn spawn(self, me: Handle<Counter>, s: &mut Spawner<Counter>) -> Counter {
        s.on(me, |ctx: &mut Context<Counter>, _: &Bump| {
            ctx.me().0 += 1;
            if let Some(mut clock) = ctx.resource_mut::<Clock>() {
                clock.0 += 1;
            }
        });
        Counter(0)
    }
}

/// A builder that reads config while spawning.
struct Label(&'static str);
impl Widget for Label {}

struct LabelBuilder;
impl WidgetBuild for LabelBuilder {
    type Widget = Label;
    fn spawn(self, _me: Handle<Label>, s: &mut Spawner<Label>) -> Label {
        Label(s.resource::<Theme>().map(|t| t.0).unwrap_or("none"))
    }
}

// ── basics ───────────────────────────────────────────────────────────────────

#[test]
fn absent_resource_reads_as_none() {
    let mut app = App::new();
    assert!(app.resource::<Clock>().is_none());
    assert!(app.resource_mut::<Clock>().is_none());
    assert_eq!(app.remove_resource::<Clock>(), None);
}

#[test]
fn insert_then_read_then_write_then_remove() {
    let mut app = App::new();
    app.insert_resource(Clock(1));
    assert_eq!(app.resource::<Clock>().unwrap().0, 1);

    app.resource_mut::<Clock>().unwrap().0 = 7;
    assert_eq!(app.resource::<Clock>().unwrap().0, 7);

    assert_eq!(app.remove_resource::<Clock>(), Some(Clock(7)));
    assert!(app.resource::<Clock>().is_none());
}

#[test]
fn resources_are_keyed_by_type() {
    let mut app = App::new();
    app.insert_resource(Clock(1));
    app.insert_resource(Theme("dark"));
    assert_eq!(app.resource::<Clock>().unwrap().0, 1);
    assert_eq!(app.resource::<Theme>().unwrap().0, "dark");
}

#[test]
#[should_panic(expected = "already present")]
fn duplicate_insert_panics() {
    let mut app = App::new();
    app.insert_resource(Clock(1));
    app.insert_resource(Clock(2));
}

#[test]
fn removed_resource_can_be_inserted_again() {
    let mut app = App::new();
    app.insert_resource(Clock(1));
    app.remove_resource::<Clock>().unwrap();
    app.insert_resource(Clock(2));
    assert_eq!(app.resource::<Clock>().unwrap().0, 2);
}

// ── readers ──────────────────────────────────────────────────────────────────

#[test]
fn many_readers_coexist() {
    let mut app = App::new();
    app.insert_resource(Clock(3));
    let a = app.resource::<Clock>().unwrap();
    let b = app.resource::<Clock>().unwrap();
    assert_eq!((a.0, b.0), (3, 3));
}

#[test]
fn a_live_reader_blocks_writes_and_removal_only() {
    let mut app = App::new();
    app.insert_resource(Clock(3));
    let reader = app.resource::<Clock>().unwrap();

    assert!(app.resource_mut::<Clock>().is_none());
    assert_eq!(
        app.remove_resource::<Clock>(),
        None,
        "refused, not half done"
    );
    assert_eq!(
        app.resource::<Clock>().unwrap().0,
        3,
        "still present after refusal"
    );

    drop(reader);
    assert!(app.resource_mut::<Clock>().is_some());
    assert_eq!(app.remove_resource::<Clock>(), Some(Clock(3)));
}

#[test]
fn a_reader_does_not_borrow_the_app() {
    let mut app = App::new();
    app.insert_resource(Clock(3));
    let clock = app.resource::<Clock>().unwrap();
    // Tree and queues stay usable while the proxy lives.
    let c = app.spawn(app.root(), CounterBuilder).unwrap();
    app.emit(Bump, &[c.id()]);
    assert_eq!(clock.0, 3);
}

// ── writers ──────────────────────────────────────────────────────────────────

#[test]
fn a_live_writer_hides_the_resource_from_everyone() {
    let mut app = App::new();
    app.insert_resource(Clock(3));
    let mut writer = app.resource_mut::<Clock>().unwrap();
    writer.0 = 4;

    assert!(app.resource::<Clock>().is_none());
    assert!(app.resource_mut::<Clock>().is_none());
    assert_eq!(app.remove_resource::<Clock>(), None);

    drop(writer);
    assert_eq!(
        app.resource::<Clock>().unwrap().0,
        4,
        "written value came back"
    );
}

#[test]
#[should_panic(expected = "already present")]
fn insert_while_a_writer_is_out_panics() {
    let mut app = App::new();
    app.insert_resource(Clock(3));
    let _writer = app.resource_mut::<Clock>().unwrap();
    app.insert_resource(Clock(9));
}

#[test]
fn a_writer_does_not_borrow_the_app() {
    let mut app = App::new();
    app.insert_resource(Clock(0));
    let mut clock = app.resource_mut::<Clock>().unwrap();
    for _ in 0..3 {
        app.spawn(app.root(), CounterBuilder).unwrap();
        clock.0 += 1;
    }
    drop(clock);
    assert_eq!(app.widgets::<Counter>().count(), 3);
    assert_eq!(app.resource::<Clock>().unwrap().0, 3);
}

#[test]
fn a_writer_is_put_back_on_unwind() {
    let mut app = App::new();
    app.insert_resource(Clock(1));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut clock = app.resource_mut::<Clock>().unwrap();
        clock.0 = 2;
        panic!("boom");
    }));
    assert!(result.is_err());
    assert_eq!(app.resource::<Clock>().unwrap().0, 2);
}

#[test]
fn a_writer_outliving_the_app_is_harmless() {
    let mut app = App::new();
    app.insert_resource(Clock(1));
    let writer = app.resource_mut::<Clock>().unwrap();
    drop(app);
    assert_eq!(writer.0, 1);
    drop(writer);
}

// ── from handlers, systems, and builders ─────────────────────────────────────

#[test]
fn handlers_reach_resources() {
    let mut app = App::new();
    app.insert_resource(Clock(0));
    let a = app.spawn(app.root(), CounterBuilder).unwrap();
    let b = app.spawn(app.root(), CounterBuilder).unwrap();
    app.emit(Bump, &[a.id(), b.id(), a.id()]);
    app.flush();
    assert_eq!(app.widget::<Counter>(a).unwrap().0, 2);
    assert_eq!(app.widget::<Counter>(b).unwrap().0, 1);
    assert_eq!(app.resource::<Clock>().unwrap().0, 3);
}

#[test]
fn a_handler_sees_none_while_a_system_holds_the_writer() {
    fn on_sync(app: &mut App, _: &Sync) {
        let clock = app.resource_mut::<Clock>().unwrap();
        // Runs the handler *inside* the writer's lifetime: no clock for it.
        let targets: Vec<_> = app.widgets::<Counter>().map(|(id, _)| id).collect();
        app.emit(Bump, &targets);
        app.flush();
        drop(clock);
    }
    let mut app = App::new();
    app.insert_resource(Clock(0));
    app.system(on_sync);
    let c = app.spawn(app.root(), CounterBuilder).unwrap();
    app.signal(Sync);
    app.flush();
    assert_eq!(app.widget::<Counter>(c).unwrap().0, 1, "handler still ran");
    assert_eq!(app.resource::<Clock>().unwrap().0, 0, "but could not stamp");
}

#[test]
fn builders_read_resources_while_spawning() {
    let mut app = App::new();
    let bare = app.spawn(app.root(), LabelBuilder).unwrap();
    app.insert_resource(Theme("dark"));
    let themed = app.spawn(app.root(), LabelBuilder).unwrap();
    assert_eq!(app.widget::<Label>(bare).unwrap().0, "none");
    assert_eq!(app.widget::<Label>(themed).unwrap().0, "dark");
}

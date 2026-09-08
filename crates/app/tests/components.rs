use app::prelude::*;

// ── fixtures ─────────────────────────────────────────────────────────────────

#[derive(Debug, Default, PartialEq, Clone, Copy)]
struct Pos(i32, i32);
impl Component for Pos {}

#[derive(Debug, Default, PartialEq)]
struct Tag(Option<&'static str>);
impl Component for Tag {}

struct Counter(u32);
impl Widget for Counter {}

struct Bump;
impl Event for Bump {}

struct Sync;
impl Signal for Sync {}

/// Every handler run bumps the widget, and moves its `Pos` if it can.
struct CounterBuilder;
impl WidgetBuild for CounterBuilder {
    type Widget = Counter;
    fn spawn(self, me: Handle<Counter>, s: &mut Spawner<Counter>) -> Counter {
        s.on(me, |ctx: &mut Context<Counter>, _: &Bump| {
            ctx.me().0 += 1;
            if let Some(mut pos) = ctx.component_mut::<Pos>() {
                pos.0 += 1;
            }
        });
        Counter(0)
    }
}

/// A builder that configures its own node.
struct Placed(i32, i32);
impl WidgetBuild for Placed {
    type Widget = Counter;
    fn spawn(self, _me: Handle<Counter>, s: &mut Spawner<Counter>) -> Counter {
        let before = s.component::<Pos>().map(|p| *p);
        assert_eq!(before, Some(Pos::default()), "fresh node starts at default");
        s.set_component(Pos(self.0, self.1));
        Counter(0)
    }
}

/// A widget that records whether its builder could see its `Pos`.
struct Seen(bool);
impl Widget for Seen {}

struct ProbeBuilder;
impl WidgetBuild for ProbeBuilder {
    type Widget = Seen;
    fn spawn(self, _me: Handle<Seen>, s: &mut Spawner<Seen>) -> Seen {
        Seen(s.component::<Pos>().is_some())
    }
}

// ── registration ─────────────────────────────────────────────────────────────

#[test]
fn register_backfills_existing_nodes_and_root() {
    let mut app = App::new();
    let a = app.spawn(app.root(), CounterBuilder).unwrap();
    app.register_component::<Pos>();
    let pos = app.components::<Pos>().unwrap();
    assert_eq!(pos.get(app.root()), Some(&Pos::default()));
    assert_eq!(pos.get(a), Some(&Pos::default()));
    assert_eq!(pos.iter().count(), 2);
}

#[test]
#[should_panic(expected = "already registered")]
fn duplicate_register_panics() {
    let mut app = App::new();
    app.register_component::<Pos>().register_component::<Pos>();
}

#[test]
#[should_panic(expected = "not registered")]
fn unregistered_read_panics() {
    let app = App::new();
    let _ = app.components::<Pos>();
}

#[test]
#[should_panic(expected = "not registered")]
fn unregistered_write_panics() {
    let mut app = App::new();
    let _ = app.components_mut::<Pos>();
}

#[test]
fn columns_are_keyed_by_type() {
    let mut app = App::new();
    app.register_component::<Pos>().register_component::<Tag>();
    let a = app.spawn(app.root(), CounterBuilder).unwrap();
    app.components_mut::<Pos>().unwrap()[a] = Pos(1, 2);
    app.components_mut::<Tag>().unwrap()[a] = Tag(Some("a"));
    assert_eq!(app.components::<Pos>().unwrap()[a], Pos(1, 2));
    assert_eq!(app.components::<Tag>().unwrap()[a], Tag(Some("a")));
}

// ── lifecycle ────────────────────────────────────────────────────────────────

#[test]
fn spawn_gets_default_and_remove_resets() {
    let mut app = App::new();
    app.register_component::<Pos>();
    let a = app.spawn(app.root(), CounterBuilder).unwrap();
    assert_eq!(app.components::<Pos>().unwrap()[a], Pos::default());

    app.components_mut::<Pos>().unwrap()[a] = Pos(5, 5);
    app.remove(a).unwrap();
    assert_eq!(app.components::<Pos>().unwrap().get(a), None, "stale");

    // The slot is reused; the new node must not inherit `Pos(5, 5)`.
    let b = app.spawn(app.root(), CounterBuilder).unwrap();
    assert_eq!(b.id().component_index(), a.id().component_index());
    assert_eq!(app.components::<Pos>().unwrap()[b], Pos::default());
    assert_eq!(app.components::<Pos>().unwrap().get(a), None);
}

#[test]
fn removing_a_subtree_resets_every_node_in_it() {
    struct Nest;
    impl WidgetBuild for Nest {
        type Widget = Counter;
        fn spawn(self, me: Handle<Counter>, s: &mut Spawner<Counter>) -> Counter {
            s.child(me, Placed(1, 1));
            s.child(me, Placed(2, 2));
            Counter(0)
        }
    }
    let mut app = App::new();
    app.register_component::<Pos>();
    let top = app.spawn(app.root(), Nest).unwrap();
    assert_eq!(app.components::<Pos>().unwrap().iter().count(), 4);
    app.remove(top).unwrap();
    let pos = app.components::<Pos>().unwrap();
    assert_eq!(pos.iter().count(), 1, "only the root is left");
    assert_eq!(pos.get(app.root()), Some(&Pos::default()));
}

#[test]
fn iteration_visits_live_nodes_only() {
    let mut app = App::new();
    app.register_component::<Pos>();
    let a = app.spawn(app.root(), Placed(1, 0)).unwrap();
    let b = app.spawn(app.root(), Placed(2, 0)).unwrap();
    let c = app.spawn(app.root(), Placed(3, 0)).unwrap();
    app.remove(b).unwrap();

    let mut seen: Vec<_> = app
        .components::<Pos>()
        .unwrap()
        .iter()
        .map(|(id, p)| (id, *p))
        .collect();
    seen.sort_by_key(|(id, _)| id.component_index());
    assert_eq!(
        seen,
        vec![
            (app.root(), Pos::default()),
            (a.id(), Pos(1, 0)),
            (c.id(), Pos(3, 0)),
        ]
    );

    for (_, p) in app.components_mut::<Pos>().unwrap().iter_mut() {
        p.1 = 9;
    }
    assert_eq!(app.components::<Pos>().unwrap()[a], Pos(1, 9));
}

#[test]
#[should_panic(expected = "stale node id")]
fn indexing_a_stale_id_panics() {
    let mut app = App::new();
    app.register_component::<Pos>();
    let a = app.spawn(app.root(), CounterBuilder).unwrap();
    app.remove(a).unwrap();
    let _ = app.components::<Pos>().unwrap()[a];
}

// ── builders and handlers ────────────────────────────────────────────────────

#[test]
fn builders_configure_their_own_node() {
    let mut app = App::new();
    app.register_component::<Pos>();
    let a = app.spawn(app.root(), Placed(3, 4)).unwrap();
    assert_eq!(app.components::<Pos>().unwrap()[a], Pos(3, 4));
}

#[test]
fn set_component_returns_the_old_value() {
    struct Twice;
    impl WidgetBuild for Twice {
        type Widget = Counter;
        fn spawn(self, _me: Handle<Counter>, s: &mut Spawner<Counter>) -> Counter {
            assert_eq!(s.set_component(Pos(1, 1)), Some(Pos::default()));
            assert_eq!(s.set_component(Pos(2, 2)), Some(Pos(1, 1)));
            Counter(0)
        }
    }
    let mut app = App::new();
    app.register_component::<Pos>();
    let a = app.spawn(app.root(), Twice).unwrap();
    assert_eq!(app.components::<Pos>().unwrap()[a], Pos(2, 2));
}

#[test]
fn handlers_read_and_write_their_own_component() {
    let mut app = App::new();
    app.register_component::<Pos>();
    let a = app.spawn(app.root(), CounterBuilder).unwrap();
    let b = app.spawn(app.root(), CounterBuilder).unwrap();
    app.emit(Bump, &[a.id(), b.id(), a.id()]);
    app.flush();
    let pos = app.components::<Pos>().unwrap();
    assert_eq!(pos[a], Pos(2, 0));
    assert_eq!(pos[b], Pos(1, 0));
    assert_eq!(app.widget::<Counter>(a).unwrap().0, 2);
}

#[test]
fn a_handler_sees_none_while_a_system_holds_the_writer() {
    fn on_sync(app: &mut App, _: &Sync) {
        let column = app.components_mut::<Pos>().unwrap();
        let targets: Vec<_> = app.widgets::<Counter>().map(|(id, _)| id).collect();
        app.emit(Bump, &targets);
        app.flush();
        drop(column);
    }
    let mut app = App::new();
    app.register_component::<Pos>();
    app.system(on_sync);
    let a = app.spawn(app.root(), CounterBuilder).unwrap();
    app.signal(Sync);
    app.flush();
    assert_eq!(app.widget::<Counter>(a).unwrap().0, 1, "handler still ran");
    assert_eq!(
        app.components::<Pos>().unwrap()[a],
        Pos(0, 0),
        "but could not move"
    );
}

#[test]
fn a_single_node_proxy_counts_as_holding_the_column() {
    struct Holder;
    impl WidgetBuild for Holder {
        type Widget = Counter;
        fn spawn(self, _me: Handle<Counter>, s: &mut Spawner<Counter>) -> Counter {
            let reader = s.component::<Pos>().unwrap();
            assert!(s.component_mut::<Pos>().is_none(), "reader blocks writer");
            assert_eq!(*reader, Pos::default());
            drop(reader);

            let writer = s.component_mut::<Pos>().unwrap();
            assert!(s.component::<Pos>().is_none(), "writer blocks reader");
            assert!(s.component_mut::<Pos>().is_none(), "writer blocks writer");
            drop(writer);
            Counter(0)
        }
    }
    let mut app = App::new();
    app.register_component::<Pos>();
    app.spawn(app.root(), Holder).unwrap();
}

// ── readers ──────────────────────────────────────────────────────────────────

#[test]
fn many_readers_coexist_and_block_the_writer() {
    let mut app = App::new();
    app.register_component::<Pos>();
    let a = app.components::<Pos>().unwrap();
    let b = app.components::<Pos>().unwrap();
    assert!(app.components_mut::<Pos>().is_none());
    drop(a);
    assert!(
        app.components_mut::<Pos>().is_none(),
        "one reader is enough"
    );
    drop(b);
    assert!(app.components_mut::<Pos>().is_some());
}

#[test]
fn a_reader_does_not_see_nodes_spawned_meanwhile() {
    let mut app = App::new();
    app.register_component::<Pos>();
    let reader = app.components::<Pos>().unwrap();
    let a = app.spawn(app.root(), CounterBuilder).unwrap();
    assert_eq!(reader.get(a), None, "not visible to a column taken earlier");
    assert_eq!(
        app.components::<Pos>().unwrap().get(a),
        None,
        "nor to a new reader while the first is alive"
    );
    drop(reader);
    assert_eq!(app.components::<Pos>().unwrap()[a], Pos::default());
}

#[test]
fn a_builder_sees_none_while_the_column_is_shared() {
    let mut app = App::new();
    app.register_component::<Pos>();
    let reader = app.components::<Pos>().unwrap();
    let blind = app.spawn(app.root(), ProbeBuilder).unwrap();
    drop(reader);
    let sighted = app.spawn(app.root(), ProbeBuilder).unwrap();
    assert!(!app.widget::<Seen>(blind).unwrap().0);
    assert!(app.widget::<Seen>(sighted).unwrap().0);
}

// ── writers ──────────────────────────────────────────────────────────────────

#[test]
fn a_live_writer_hides_the_column_from_everyone() {
    let mut app = App::new();
    app.register_component::<Pos>();
    let root = app.root();
    let mut writer = app.components_mut::<Pos>().unwrap();
    writer[root] = Pos(7, 7);
    assert!(app.components::<Pos>().is_none());
    assert!(app.components_mut::<Pos>().is_none());
    drop(writer);
    assert_eq!(
        app.components::<Pos>().unwrap()[root],
        Pos(7, 7),
        "written value came back"
    );
}

#[test]
fn spawns_and_removes_during_a_write_are_applied_on_drop() {
    let mut app = App::new();
    app.register_component::<Pos>();
    let old = app.spawn(app.root(), CounterBuilder).unwrap();

    let mut writer = app.components_mut::<Pos>().unwrap();
    writer[old] = Pos(1, 1);
    app.remove(old).unwrap();
    let new = app.spawn(app.root(), CounterBuilder).unwrap();
    assert_eq!(
        new.id().component_index(),
        old.id().component_index(),
        "slot reused"
    );
    let extra = app.spawn(app.root(), CounterBuilder).unwrap();
    assert_eq!(writer.get(new), None, "the writer still sees the old node");
    assert_eq!(writer.get(extra), None, "and nothing beyond its length");
    assert_eq!(writer[old], Pos(1, 1));
    drop(writer);

    let pos = app.components::<Pos>().unwrap();
    assert_eq!(pos.get(old), None);
    assert_eq!(pos.get(new), Some(&Pos::default()), "reused slot was reset");
    assert_eq!(pos.get(extra), Some(&Pos::default()), "column grew");
    assert_eq!(pos.iter().count(), 3);
}

#[test]
fn a_writer_is_put_back_on_unwind() {
    let mut app = App::new();
    app.register_component::<Pos>();
    let root = app.root();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut writer = app.components_mut::<Pos>().unwrap();
        writer[root] = Pos(2, 2);
        panic!("boom");
    }));
    assert!(result.is_err());
    assert_eq!(app.components::<Pos>().unwrap()[root], Pos(2, 2));
}

#[test]
fn a_writer_outliving_the_app_is_harmless() {
    let mut app = App::new();
    app.register_component::<Pos>();
    let root = app.root();
    let writer = app.components_mut::<Pos>().unwrap();
    drop(app);
    assert_eq!(writer[root], Pos::default());
    drop(writer);
}

use app::Error;
use app::prelude::*;

// ── fixtures ─────────────────────────────────────────────────────────────────

struct Div;
impl Widget for Div {}

struct Text(String);
impl Widget for Text {}

struct DivBuilder;
impl WidgetBuild for DivBuilder {
    type Widget = Div;
    fn spawn(self, _me: NodeId, _s: &mut Spawner) -> Div {
        Div
    }
}

struct TextBuilder(&'static str);
impl WidgetBuild for TextBuilder {
    type Widget = Text;
    fn spawn(self, _me: NodeId, _s: &mut Spawner) -> Text {
        Text(self.0.to_owned())
    }
}

/// `Div` > `Div` > `Text`, exercising both `me` and an id returned by `child`.
struct LabelBuilder(&'static str);
impl WidgetBuild for LabelBuilder {
    type Widget = Div;
    fn spawn(self, me: NodeId, s: &mut Spawner) -> Div {
        let inner = s.child(me, DivBuilder);
        s.child(inner, TextBuilder(self.0));
        Div
    }
}

fn ids<W: Widget>(app: &mut App) -> Vec<NodeId> {
    let mut v: Vec<_> = app.widgets::<W>().map(|(id, _)| id).collect();
    v.sort();
    v
}

/// An id that was live once and isn't any more.
fn stale(app: &mut App) -> NodeId {
    let id = app.spawn(app.root(), DivBuilder).unwrap();
    app.remove(id).unwrap();
    id
}

// ── root ─────────────────────────────────────────────────────────────────────

#[test]
fn root_is_its_own_parent_and_has_no_widget() {
    let app = App::new();
    let root = app.root();
    assert_eq!(app.parent(root), Some(root));
    assert_eq!(app.children(root), Some(&[][..]));
    assert!(!root.has_widget());
    assert!(app.widget::<Div>(root).is_none());
}

#[test]
fn root_cannot_be_removed() {
    let mut app = App::new();
    assert_eq!(app.remove(app.root()), Err(Error::Root));
    assert_eq!(app.parent(app.root()), Some(app.root()));
}

// ── spawn ────────────────────────────────────────────────────────────────────

#[test]
fn spawn_appends_in_sibling_order() {
    let mut app = App::new();
    let root = app.root();
    let a = app.spawn(root, DivBuilder).unwrap();
    let b = app.spawn(root, TextBuilder("b")).unwrap();
    let c = app.spawn(a, DivBuilder).unwrap();

    assert_eq!(app.children(root), Some(&[a, b][..]));
    assert_eq!(app.children(a), Some(&[c][..]));
    assert_eq!(app.parent(a), Some(root));
    assert_eq!(app.parent(c), Some(a));
    assert!(a.has_widget());
}

#[test]
fn spawn_under_stale_parent_fails() {
    let mut app = App::new();
    let a = stale(&mut app);
    assert_eq!(app.spawn(a, DivBuilder), Err(Error::Stale));
    assert_eq!(app.children(app.root()), Some(&[][..]));
}

#[test]
fn builder_receives_its_final_id() {
    struct Probe;
    impl Widget for Probe {}
    struct ProbeBuilder(std::rc::Rc<std::cell::Cell<Option<NodeId>>>);
    impl WidgetBuild for ProbeBuilder {
        type Widget = Probe;
        fn spawn(self, me: NodeId, _s: &mut Spawner) -> Probe {
            self.0.set(Some(me));
            Probe
        }
    }

    let seen = std::rc::Rc::new(std::cell::Cell::new(None));
    let mut app = App::new();
    let id = app.spawn(app.root(), ProbeBuilder(seen.clone())).unwrap();
    assert_eq!(seen.get(), Some(id));
}

#[test]
fn composite_builder_spawns_a_subtree() {
    let mut app = App::new();
    let label = app.spawn(app.root(), LabelBuilder("hi")).unwrap();

    let inner = app.children(label).unwrap()[0];
    let text = app.children(inner).unwrap()[0];
    assert_eq!(app.children(label).unwrap().len(), 1);
    assert_eq!(app.children(inner).unwrap().len(), 1);
    assert_eq!(app.parent(text), Some(inner));
    assert_eq!(app.parent(inner), Some(label));
    assert_eq!(app.widget::<Text>(text).unwrap().0, "hi");
    assert!(app.widget::<Div>(inner).is_some());
}

// ── widget access ────────────────────────────────────────────────────────────

#[test]
fn widget_and_widget_mut_resolve_by_type() {
    let mut app = App::new();
    let t = app.spawn(app.root(), TextBuilder("a")).unwrap();

    assert_eq!(app.widget::<Text>(t).unwrap().0, "a");
    app.widget_mut::<Text>(t).unwrap().0.push('b');
    assert_eq!(app.widget::<Text>(t).unwrap().0, "ab");

    // Wrong type → None, no panic.
    assert!(app.widget::<Div>(t).is_none());
    assert!(app.widget_mut::<Div>(t).is_none());
}

#[test]
fn widgets_iterates_one_column() {
    let mut app = App::new();
    let root = app.root();
    let d1 = app.spawn(root, DivBuilder).unwrap();
    let t1 = app.spawn(root, TextBuilder("1")).unwrap();
    let d2 = app.spawn(d1, DivBuilder).unwrap();
    let t2 = app.spawn(d2, TextBuilder("2")).unwrap();

    let mut divs = vec![d1, d2];
    divs.sort();
    let mut texts = vec![t1, t2];
    texts.sort();
    assert_eq!(ids::<Div>(&mut app), divs);
    assert_eq!(ids::<Text>(&mut app), texts);

    for (_, text) in app.widgets::<Text>() {
        text.0.push('!');
    }
    assert_eq!(app.widget::<Text>(t1).unwrap().0, "1!");
    assert_eq!(app.widget::<Text>(t2).unwrap().0, "2!");
}

#[test]
fn widgets_of_unseen_type_is_empty() {
    struct Never;
    impl Widget for Never {}
    let mut app = App::new();
    app.spawn(app.root(), DivBuilder).unwrap();
    assert_eq!(app.widgets::<Never>().count(), 0);
}

// ── remove ───────────────────────────────────────────────────────────────────

#[test]
fn remove_frees_the_whole_subtree() {
    let mut app = App::new();
    let root = app.root();
    let keep = app.spawn(root, DivBuilder).unwrap();
    let label = app.spawn(root, LabelBuilder("x")).unwrap();
    let inner = app.children(label).unwrap()[0];
    let text = app.children(inner).unwrap()[0];

    app.remove(label).unwrap();

    for id in [label, inner, text] {
        assert_eq!(app.parent(id), None, "{id:?} should be stale");
        assert_eq!(app.children(id), None);
        assert!(app.widget::<Div>(id).is_none());
        assert!(app.widget::<Text>(id).is_none());
    }
    assert_eq!(app.children(root), Some(&[keep][..]));
    assert_eq!(ids::<Div>(&mut app), vec![keep]);
    assert_eq!(ids::<Text>(&mut app), vec![]);
}

#[test]
fn remove_twice_is_stale() {
    let mut app = App::new();
    let a = app.spawn(app.root(), DivBuilder).unwrap();
    assert_eq!(app.remove(a), Ok(()));
    assert_eq!(app.remove(a), Err(Error::Stale));
}

#[test]
fn reused_slot_gets_a_new_generation_and_old_id_stays_stale() {
    let mut app = App::new();
    let a = app.spawn(app.root(), TextBuilder("old")).unwrap();
    app.remove(a).unwrap();
    let b = app.spawn(app.root(), TextBuilder("new")).unwrap();

    // Both free lists are LIFO, so the same slots come back…
    assert_eq!(a.component_index(), b.component_index());
    assert_eq!(a.widget_index(), b.widget_index());
    assert_eq!(a.widget_column(), b.widget_column());
    // …under a fresh generation.
    assert_ne!(a, b);
    assert_eq!(b.generation(), a.generation() + 1);

    assert!(app.widget::<Text>(a).is_none());
    assert_eq!(app.widget::<Text>(b).unwrap().0, "new");
    assert_eq!(app.parent(a), None);
}

#[test]
fn subtree_slots_are_all_reused() {
    let mut app = App::new();
    let root = app.root();
    let label = app.spawn(root, LabelBuilder("x")).unwrap();
    let inner = app.children(label).unwrap()[0];
    let text = app.children(inner).unwrap()[0];
    app.remove(label).unwrap();

    // Three node slots and three widget slots were freed; three fresh spawns
    // must consume exactly those (no arena growth, no leaked widget slot).
    let mut node_slots = vec![
        label.component_index(),
        inner.component_index(),
        text.component_index(),
    ];
    node_slots.sort();
    let d1 = app.spawn(root, DivBuilder).unwrap();
    let d2 = app.spawn(root, DivBuilder).unwrap();
    let t = app.spawn(root, TextBuilder("y")).unwrap();
    let mut got = vec![
        d1.component_index(),
        d2.component_index(),
        t.component_index(),
    ];
    got.sort();
    assert_eq!(got, node_slots);

    let mut div_slots = vec![label.widget_index(), inner.widget_index()];
    div_slots.sort();
    let mut got = vec![d1.widget_index(), d2.widget_index()];
    got.sort();
    assert_eq!(got, div_slots);
    assert_eq!(t.widget_index(), text.widget_index());
}

// ── failed builds ────────────────────────────────────────────────────────────

/// Attaches one good child under `me`, then one under a stale id.
struct BadParent(NodeId);
impl WidgetBuild for BadParent {
    type Widget = Div;
    fn spawn(self, me: NodeId, s: &mut Spawner) -> Div {
        s.child(me, DivBuilder);
        s.child(self.0, TextBuilder("x"));
        Div
    }
}

#[test]
fn failed_child_tears_down_the_partial_build() {
    let mut app = App::new();
    let root = app.root();
    let gone = stale(&mut app);
    let keep = app.spawn(root, DivBuilder).unwrap();

    assert_eq!(app.spawn(root, BadParent(gone)), Err(Error::Stale));

    assert_eq!(app.children(root), Some(&[keep][..]));
    assert_eq!(ids::<Div>(&mut app), vec![keep]);
    assert_eq!(app.widgets::<Text>().count(), 0);

    // The slots the failed build reserved were released: the next spawn
    // reuses rather than grows.
    let next = app.spawn(root, DivBuilder).unwrap();
    assert!(next.component_index() <= keep.component_index() + 2);
}

#[test]
fn child_after_failure_is_a_noop() {
    struct Greedy(NodeId);
    impl WidgetBuild for Greedy {
        type Widget = Div;
        fn spawn(self, me: NodeId, s: &mut Spawner) -> Div {
            let a = s.child(me, DivBuilder);
            let b = s.child(self.0, DivBuilder);
            let c = s.child(a, DivBuilder);
            assert_ne!(a, b);
            // Once `b` failed, `c` must not be attempted; it gets the same
            // matches-nothing id.
            assert_eq!(b, c);
            assert_ne!(c, a);
            Div
        }
    }

    let mut app = App::new();
    let root = app.root();
    let gone = stale(&mut app);
    assert_eq!(app.spawn(root, Greedy(gone)), Err(Error::Stale));
    assert_eq!(app.children(root), Some(&[][..]));
    assert_eq!(app.widgets::<Div>().count(), 0);
}

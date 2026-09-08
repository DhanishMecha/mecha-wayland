//! `Paint` as a component, its primitives, and the tick that reports which
//! nodes' paint differs from the last.

use app::prelude::*;
use assets::{AtlasId, SpriteRegion};
use paint::prelude::*;
use utils::{Point, Size};

// ── fixtures ─────────────────────────────────────────────────────────────────

struct Node;
impl Widget for Node {}

struct Painted(Paint);
impl WidgetBuild for Painted {
    type Widget = Node;
    fn spawn(self, _: Handle<Node>, s: &mut Spawner<Node>) -> Node {
        s.set_component(self.0);
        Node
    }
}

/// Every `PaintChanged` signalled, in order.
#[derive(Default)]
struct Changes(Vec<Vec<NodeId>>);
impl Resource for Changes {}

fn record(app: &mut App, c: &PaintChanged) {
    app.resource_mut::<Changes>().unwrap().0.push(c.0.clone());
}

fn app() -> App {
    let mut app = App::new();
    app.add_module(PaintModule)
        .insert_resource(Changes::default())
        .system(record);
    app
}

/// One batch end: the nodes reported changed, or none.
fn tick(app: &mut App) -> Vec<NodeId> {
    let before = app.resource::<Changes>().unwrap().0.len();
    app.signal(Tick);
    app.flush();
    let changes = app.resource::<Changes>().unwrap();
    match changes.0.len() - before {
        0 => Vec::new(),
        1 => changes.0.last().unwrap().clone(),
        n => panic!("{n} PaintChanged in one tick"),
    }
}

fn set_paint(app: &mut App, id: impl Into<NodeId>, paint: Paint) {
    app.components_mut::<Paint>().unwrap()[id] = paint;
}

const RED: Color = Color::rgb(1.0, 0.0, 0.0);
const BLUE: Color = Color::rgb(0.0, 0.0, 1.0);

fn sprite(color: Color) -> MonochromeSprite {
    MonochromeSprite {
        atlas: AtlasId(1),
        region: SpriteRegion {
            x: 0.0,
            y: 0.0,
            w: 8.0,
            h: 8.0,
        },
        offset: Point::ZERO,
        size: Size::new(8.0, 8.0),
        color,
    }
}

// ── primitives ───────────────────────────────────────────────────────────────

#[test]
fn quad_verbs_compose_by_value() {
    let q = Quad::new(RED).radius(4.0).border(2.0, BLUE);
    assert_eq!(q.color, RED);
    assert_eq!(q.radius, 4.0);
    assert_eq!(
        q.border,
        Some(Border {
            width: 2.0,
            color: BLUE
        })
    );
    assert_eq!(q.color(BLUE).color, BLUE);
}

#[test]
fn a_default_quad_draws_nothing() {
    assert!(Quad::default().is_invisible());
    assert!(Paint::Quad(Quad::default()).is_invisible());
    assert!(Paint::None.is_invisible());
    assert!(!Quad::new(RED).is_invisible());
    assert!(
        Quad::new(Color::TRANSPARENT)
            .border(0.0, RED)
            .is_invisible()
    );
    assert!(
        Quad::new(Color::TRANSPARENT)
            .border(1.0, Color::TRANSPARENT)
            .is_invisible()
    );
    assert!(
        !Quad::new(Color::TRANSPARENT)
            .border(1.0, RED)
            .is_invisible()
    );
}

#[test]
fn sprites_are_invisible_when_every_one_is() {
    assert!(Paint::Sprites(Vec::new()).is_invisible());
    assert!(sprite(Color::TRANSPARENT).is_invisible());
    assert!(
        MonochromeSprite {
            size: Size::ZERO,
            ..sprite(RED)
        }
        .is_invisible()
    );
    assert!(
        Paint::Sprites(vec![sprite(Color::TRANSPARENT), sprite(Color::TRANSPARENT)]).is_invisible()
    );
    assert!(!Paint::Sprites(vec![sprite(Color::TRANSPARENT), sprite(RED)]).is_invisible());
}

#[test]
fn every_node_defaults_to_none() {
    let mut app = app();
    let node = app
        .spawn(app.root(), Painted(Paint::Quad(Quad::new(RED))))
        .unwrap();
    let paints = app.components::<Paint>().unwrap();
    assert_eq!(paints[app.root()], Paint::None);
    assert_eq!(paints[node], Paint::Quad(Quad::new(RED)));
}

// ── PaintChanged ─────────────────────────────────────────────────────────────

#[test]
fn the_first_tick_reports_what_was_painted_at_spawn() {
    let mut app = app();
    let quad = app
        .spawn(app.root(), Painted(Paint::Quad(Quad::new(RED))))
        .unwrap();
    let none = app.spawn(app.root(), Painted(Paint::None)).unwrap();
    let run = app
        .spawn(app.root(), Painted(Paint::Sprites(vec![sprite(RED)])))
        .unwrap();

    let changed = tick(&mut app);
    assert_eq!(changed, [quad.id(), run.id()]);
    assert!(
        !changed.contains(&none.id()),
        "a paint that is still nothing is no change"
    );
    assert!(!changed.contains(&app.root()));
}

#[test]
fn a_clean_tick_reports_nothing() {
    let mut app = app();
    app.spawn(app.root(), Painted(Paint::Quad(Quad::new(RED))))
        .unwrap();
    tick(&mut app);
    assert!(tick(&mut app).is_empty());
    assert!(tick(&mut app).is_empty());
}

#[test]
fn a_write_is_reported_once_for_its_node_only() {
    let mut app = app();
    let a = app
        .spawn(app.root(), Painted(Paint::Quad(Quad::new(RED))))
        .unwrap();
    let b = app
        .spawn(app.root(), Painted(Paint::Quad(Quad::new(BLUE))))
        .unwrap();
    tick(&mut app);

    set_paint(&mut app, b, Paint::Quad(Quad::new(BLUE).radius(8.0)));
    assert_eq!(tick(&mut app), [b.id()]);
    assert!(tick(&mut app).is_empty());

    set_paint(&mut app, a, Paint::None);
    assert_eq!(
        tick(&mut app),
        [a.id()],
        "going back to nothing is a change"
    );
}

#[test]
fn writing_an_equal_paint_is_not_a_change() {
    let mut app = app();
    let a = app
        .spawn(app.root(), Painted(Paint::Quad(Quad::new(RED))))
        .unwrap();
    tick(&mut app);
    set_paint(&mut app, a, Paint::Quad(Quad::new(RED)));
    assert!(tick(&mut app).is_empty());
}

#[test]
fn a_changed_sprite_run_is_a_change() {
    let mut app = app();
    let run = app
        .spawn(app.root(), Painted(Paint::Sprites(vec![sprite(RED)])))
        .unwrap();
    tick(&mut app);
    set_paint(&mut app, run, Paint::Sprites(vec![sprite(BLUE)]));
    assert_eq!(tick(&mut app), [run.id()]);
    set_paint(
        &mut app,
        run,
        Paint::Sprites(vec![sprite(BLUE), sprite(BLUE)]),
    );
    assert_eq!(tick(&mut app), [run.id()]);
}

#[test]
fn several_writes_in_one_batch_are_one_signal() {
    let mut app = app();
    let a = app.spawn(app.root(), Painted(Paint::None)).unwrap();
    let b = app.spawn(app.root(), Painted(Paint::None)).unwrap();
    tick(&mut app);
    set_paint(&mut app, a, Paint::Quad(Quad::new(RED)));
    set_paint(&mut app, b, Paint::Quad(Quad::new(BLUE)));
    assert_eq!(tick(&mut app), [a.id(), b.id()], "slot order");
}

#[test]
fn a_removed_node_is_not_reported_and_its_slot_starts_clean() {
    let mut app = app();
    let a = app
        .spawn(app.root(), Painted(Paint::Quad(Quad::new(RED))))
        .unwrap();
    tick(&mut app);
    app.remove(a).unwrap();
    assert!(
        tick(&mut app).is_empty(),
        "nothing to say about a node that is gone"
    );

    // The slot is reused; a paint equal to the old occupant's is still new.
    let again = app
        .spawn(app.root(), Painted(Paint::Quad(Quad::new(RED))))
        .unwrap();
    assert_eq!(again.id().component_index(), a.id().component_index());
    assert_eq!(tick(&mut app), [again.id()]);
}

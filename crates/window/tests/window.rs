//! Windows as nodes: membership, the live list, resize, and the request
//! chain from layout and paint.

use app::prelude::*;
use layout::prelude::*;
use paint::prelude::*;
use utils::{Rect, Size};
use widgets::prelude::*;
use window::prelude::*;

// ── fixtures ─────────────────────────────────────────────────────────────────

const BG: Color = Color::rgb(0.1, 0.1, 0.12);
const RED: Color = Color::rgb(0.85, 0.25, 0.25);

/// Every `RequestFrame` and `AllWindowsClosed` since the last look.
#[derive(Default)]
struct Log {
    requests: Vec<NodeId>,
    closed: u32,
}
impl Resource for Log {}

fn on_request(app: &mut App, r: &RequestFrame) {
    app.resource_mut::<Log>().unwrap().requests.push(r.0);
}

fn on_closed(app: &mut App, _: &AllWindowsClosed) {
    app.resource_mut::<Log>().unwrap().closed += 1;
}

fn app() -> App {
    let mut app = App::new();
    app.add_module(LayoutModule)
        .add_module(PaintModule)
        .add_module(WindowModule)
        .insert_resource(Log::default())
        .system(on_request)
        .system(on_closed);
    app
}

fn tick(app: &mut App) {
    app.signal(Tick);
    app.flush();
}

/// The requests since the last call, per window in the order first asked.
fn requests(app: &mut App) -> Vec<(NodeId, usize)> {
    let asked = std::mem::take(&mut app.resource_mut::<Log>().unwrap().requests);
    let mut per: Vec<(NodeId, usize)> = Vec::new();
    for w in asked {
        match per.iter_mut().find(|(id, _)| *id == w) {
            Some((_, n)) => *n += 1,
            None => per.push((w, 1)),
        }
    }
    per
}

fn count(per: &[(NodeId, usize)], w: impl Into<NodeId>) -> usize {
    let w = w.into();
    per.iter().find(|(id, _)| *id == w).map_or(0, |(_, n)| *n)
}

fn windows(app: &App) -> Vec<NodeId> {
    app.resource::<Windows>().unwrap().iter().collect()
}

fn a_window() -> WindowBuilder {
    window()
        .title("w")
        .layout(LayoutStyle::default().size(px(480.0), px(240.0)))
        .background(Quad::new(BG))
}

/// A node outside every window, keeping handles to the window and the div
/// it spawned under it.
#[derive(Clone, Copy)]
struct Owner {
    win: Handle<Window>,
    div: Handle<Div>,
}
impl Widget for Owner {}

struct Fixture {
    owner: Handle<Owner>,
    win: Handle<Window>,
    div: Handle<Div>,
}

struct Build;
impl WidgetBuild for Build {
    type Widget = Owner;
    fn spawn(self, me: Handle<Owner>, s: &mut Spawner<Owner>) -> Owner {
        let win = s.child(me, a_window());
        let div = s.child(win, div().size(px(48.0), px(48.0)).color(RED));
        Owner { win, div }
    }
}

fn fixture(app: &mut App) -> Fixture {
    let owner = app.spawn(app.root(), Build).unwrap();
    let Owner { win, div } = *app.widget::<Owner>(owner).unwrap();
    Fixture { owner, win, div }
}

// ── W1: widget and membership ────────────────────────────────────────────────

#[test]
fn the_builder_writes_absolute_style_background_and_role() {
    let mut app = app();
    let layer = Layer {
        layer: LayerKind::Overlay,
        anchor: Anchor::TOP | Anchor::LEFT,
        exclusive_zone: 32,
        namespace: "bar".into(),
        ..Default::default()
    };
    let win = app
        .spawn(app.root(), a_window().role(Role::Layer(layer.clone())))
        .unwrap();

    let style = app.components::<LayoutStyle>().unwrap()[win].clone();
    assert_eq!(style.position, Position::Absolute);
    assert_eq!(
        (style.width, style.height),
        (px(480.0), px(240.0)),
        "the given box survives"
    );
    assert_eq!(
        app.components::<Paint>().unwrap()[win],
        Paint::Quad(Quad::new(BG))
    );
    assert_eq!(app.components::<Role>().unwrap()[win], Role::Layer(layer));
    assert_eq!(app.widget::<Window>(win).unwrap().title, "w");
}

#[test]
fn a_window_without_a_background_is_transparent_and_a_toplevel() {
    let mut app = app();
    let win = app.spawn(app.root(), window()).unwrap();
    assert_eq!(app.components::<Paint>().unwrap()[win], Paint::None);
    assert_eq!(app.components::<Role>().unwrap()[win], Role::Toplevel);
}

#[test]
fn window_of_is_the_nearest_window_at_or_above() {
    let mut app = app();
    let f = fixture(&mut app);
    let nested = app.spawn(f.div, window().title("popup")).unwrap();
    let inside_nested = app.spawn(nested, div()).unwrap();

    assert_eq!(window_of(&app, app.root()), None);
    assert_eq!(
        window_of(&app, f.owner.id()),
        None,
        "hung outside every window"
    );
    assert_eq!(
        window_of(&app, f.win.id()),
        Some(f.win.id()),
        "a window is its own"
    );
    assert_eq!(window_of(&app, f.div.id()), Some(f.win.id()));
    assert_eq!(window_of(&app, nested.id()), Some(nested.id()));
    assert_eq!(
        window_of(&app, inside_nested.id()),
        Some(nested.id()),
        "the nearest, not the outermost"
    );

    app.remove(nested).unwrap();
    assert_eq!(window_of(&app, inside_nested.id()), None, "stale");
}

#[test]
fn windows_lists_live_windows_in_spawn_order() {
    let mut app = app();
    assert!(windows(&app).is_empty());
    let a = app.spawn(app.root(), a_window()).unwrap();
    let b = app.spawn(app.root(), a_window()).unwrap();
    assert_eq!(windows(&app), [a.id(), b.id()], "listed as they spawn");

    app.remove(a).unwrap();
    tick(&mut app);
    assert_eq!(windows(&app), [b.id()], "trimmed at the batch end");
    let c = app.spawn(app.root(), a_window()).unwrap();
    assert_eq!(windows(&app), [b.id(), c.id()]);
}

#[test]
fn a_window_spawned_while_the_list_is_lent_is_listed_at_the_tick() {
    let mut app = app();
    let held = app.resource::<Windows>().unwrap();
    let a = app.spawn(app.root(), a_window()).unwrap();
    assert!(held.is_empty(), "the builder could not register");
    drop(held);
    tick(&mut app);
    assert_eq!(windows(&app), [a.id()]);
}

#[test]
fn the_window_is_laid_out_at_the_origin_at_its_requested_size() {
    let mut app = app();
    let f = fixture(&mut app);
    tick(&mut app);
    let layouts = app.components::<Layout>().unwrap();
    assert_eq!(layouts[f.win].rect, Rect::xywh(0.0, 0.0, 480.0, 240.0));
    assert_eq!(layouts[f.div].rect, Rect::xywh(0.0, 0.0, 48.0, 48.0));
    assert_eq!(
        layouts[f.owner].rect.size,
        Size::ZERO,
        "an absolute child gives its parent no size"
    );
}

// ── W2: resized ──────────────────────────────────────────────────────────────

#[test]
fn resized_rewrites_the_windows_own_size() {
    let mut app = app();
    let f = fixture(&mut app);
    tick(&mut app);

    app.emit(
        Resized {
            size: Size::new(320.0, 160.0),
        },
        &[f.win.id()],
    );
    tick(&mut app);
    let style = app.components::<LayoutStyle>().unwrap()[f.win].clone();
    assert_eq!((style.width, style.height), (px(320.0), px(160.0)));
    assert_eq!(style.position, Position::Absolute, "still absolute");
    assert_eq!(
        app.components::<Layout>().unwrap()[f.win].rect,
        Rect::xywh(0.0, 0.0, 320.0, 160.0)
    );
}

// ── the request chain ────────────────────────────────────────────────────────

#[test]
fn the_spawn_batch_requests_every_window_and_nothing_else() {
    let mut app = app();
    let f = fixture(&mut app);
    let other = app.spawn(app.root(), a_window()).unwrap();
    tick(&mut app);

    let per = requests(&mut app);
    assert!(count(&per, f.win) >= 1);
    assert!(count(&per, other) >= 1);
    assert_eq!(per.len(), 2, "only windows are asked");
}

#[test]
fn a_clean_tick_requests_nothing() {
    let mut app = app();
    fixture(&mut app);
    tick(&mut app);
    requests(&mut app);
    tick(&mut app);
    assert!(requests(&mut app).is_empty());
}

#[test]
fn a_relayout_requests_every_window() {
    let mut app = app();
    let f = fixture(&mut app);
    let other = app.spawn(app.root(), a_window()).unwrap();
    tick(&mut app);
    requests(&mut app);

    app.emit(
        Resized {
            size: Size::new(320.0, 160.0),
        },
        &[f.win.id()],
    );
    tick(&mut app);
    let per = requests(&mut app);
    assert_eq!(count(&per, f.win), 1);
    assert_eq!(count(&per, other), 1, "layout does not say what moved");
}

#[test]
fn a_paint_change_requests_only_its_window_once() {
    let mut app = app();
    let f = fixture(&mut app);
    let other = app.spawn(app.root(), a_window()).unwrap();
    tick(&mut app);
    requests(&mut app);

    app.components_mut::<Paint>().unwrap()[f.div] = Paint::Quad(Quad::new(RED).radius(8.0));
    tick(&mut app);
    let per = requests(&mut app);
    assert_eq!(count(&per, f.win), 1);
    assert_eq!(count(&per, other), 0);
}

#[test]
fn paint_changes_on_several_nodes_of_one_window_ask_once() {
    let mut app = app();
    let f = fixture(&mut app);
    let second = app.spawn(f.win, div().size(px(8.0), px(8.0))).unwrap();
    tick(&mut app);
    requests(&mut app);

    {
        let mut paints = app.components_mut::<Paint>().unwrap();
        paints[f.div] = Paint::Quad(Quad::new(RED).radius(8.0));
        paints[second] = Paint::Quad(Quad::new(RED));
    }
    tick(&mut app);
    assert_eq!(requests(&mut app), [(f.win.id(), 1)]);
}

#[test]
fn a_paint_change_outside_every_window_asks_nothing() {
    let mut app = app();
    let f = fixture(&mut app);
    let outside = app.spawn(app.root(), div().size(px(8.0), px(8.0))).unwrap();
    tick(&mut app);
    requests(&mut app);

    app.components_mut::<Paint>().unwrap()[outside] = Paint::Quad(Quad::new(RED));
    tick(&mut app);
    assert!(requests(&mut app).is_empty());
    let _ = f;
}

#[test]
fn a_spawn_and_a_remove_each_end_in_a_request() {
    let mut app = app();
    let f = fixture(&mut app);
    tick(&mut app);
    requests(&mut app);

    let extra = app.spawn(f.win, div().size(px(8.0), px(8.0))).unwrap();
    tick(&mut app);
    assert!(count(&requests(&mut app), f.win) >= 1, "spawn");

    app.remove(extra).unwrap();
    tick(&mut app);
    assert!(count(&requests(&mut app), f.win) >= 1, "remove");
}

// ── W3: lifecycle ────────────────────────────────────────────────────────────

#[test]
fn the_last_window_closing_is_signalled_once() {
    let mut app = app();
    let a = app.spawn(app.root(), a_window()).unwrap();
    let b = app.spawn(app.root(), a_window()).unwrap();
    tick(&mut app);

    app.remove(a).unwrap();
    tick(&mut app);
    assert_eq!(app.resource::<Log>().unwrap().closed, 0, "one is left");

    app.remove(b).unwrap();
    tick(&mut app);
    assert!(windows(&app).is_empty());
    assert_eq!(app.resource::<Log>().unwrap().closed, 1);
    tick(&mut app);
    assert_eq!(app.resource::<Log>().unwrap().closed, 1, "not repeated");
}

#[test]
fn an_app_that_never_had_a_window_never_closes_them_all() {
    let mut app = app();
    tick(&mut app);
    tick(&mut app);
    assert_eq!(app.resource::<Log>().unwrap().closed, 0);
}

#[test]
fn a_window_dies_with_the_subtree_that_spawned_it() {
    let mut app = app();
    let f = fixture(&mut app);
    tick(&mut app);
    app.remove(f.owner).unwrap();
    tick(&mut app);
    assert!(windows(&app).is_empty());
    assert_eq!(app.resource::<Log>().unwrap().closed, 1);
}

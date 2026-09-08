//! The pass and its dirtying, headless: a counter-shaped tree of measured
//! leaves under an absolute box, ticked by hand.

use app::prelude::*;
use layout::prelude::*;
use utils::{Rect, Size};

// ── fixtures ─────────────────────────────────────────────────────────────────

struct Node;
impl Widget for Node {}

/// A node with the given style and measure, and nothing else.
struct Styled(LayoutStyle, Measure);
impl WidgetBuild for Styled {
    type Widget = Node;
    fn spawn(self, _: Handle<Node>, s: &mut Spawner<Node>) -> Node {
        s.set_component(self.0);
        s.set_component(self.1);
        Node
    }
}

fn styled(style: LayoutStyle) -> Styled {
    Styled(style, Measure(None))
}

fn measured(w: f32, h: f32) -> Styled {
    Styled(LayoutStyle::default(), Measure(Some(Size::new(w, h))))
}

/// Every `LayoutDone` signalled, in order.
#[derive(Default)]
struct Done(Vec<LayoutDone>);
impl Resource for Done {}

fn record(app: &mut App, done: &LayoutDone) {
    app.resource_mut::<Done>().unwrap().0.push(*done);
}

fn app() -> App {
    let mut app = App::new();
    app.add_module(LayoutModule)
        .insert_resource(Done::default())
        .system(record);
    app
}

/// One batch end: what layout said.
fn tick(app: &mut App) -> LayoutDone {
    app.signal(Tick);
    app.flush();
    *app.resource::<Done>()
        .unwrap()
        .0
        .last()
        .expect("LayoutDone is always signalled")
}

fn rect(app: &App, id: impl Into<NodeId>) -> Rect {
    app.components::<Layout>().unwrap()[id].rect
}

fn set_style(app: &mut App, id: impl Into<NodeId>, style: LayoutStyle) {
    app.components_mut::<LayoutStyle>().unwrap()[id] = style;
}

/// An absolute 480x240 row, centred, gap 28, holding 64x64, 32x64, 64x64.
struct Counter {
    me: Handle<Node>,
    dec: Handle<Node>,
    label: Handle<Node>,
    inc: Handle<Node>,
}

fn counter(app: &mut App) -> Counter {
    let style = LayoutStyle::default()
        .absolute()
        .size(px(480.0), px(240.0))
        .row()
        .gap(px(28.0))
        .center();
    let me = app.spawn(app.root(), styled(style)).unwrap();
    let dec = app.spawn(me, measured(64.0, 64.0)).unwrap();
    let label = app.spawn(me, measured(32.0, 64.0)).unwrap();
    let inc = app.spawn(me, measured(64.0, 64.0)).unwrap();
    Counter {
        me,
        dec,
        label,
        inc,
    }
}

// ── L1: types and install ────────────────────────────────────────────────────

#[test]
fn every_node_has_a_default_style_after_install() {
    let mut app = app();
    let node = app
        .spawn(app.root(), Styled(LayoutStyle::default(), Measure(None)))
        .unwrap();
    let styles = app.components::<LayoutStyle>().unwrap();
    assert_eq!(styles[app.root()], LayoutStyle::default());
    assert_eq!(styles[node], LayoutStyle::default());
    assert_eq!(app.components::<Measure>().unwrap()[node], Measure(None));
    assert_eq!(app.components::<Layout>().unwrap()[node], Layout::default());
}

#[test]
fn style_verbs_compose_by_value() {
    let style = LayoutStyle::default().row().gap(px(28.0)).center();
    assert_eq!(style.direction, Direction::Row);
    assert_eq!(style.row_gap, px(28.0));
    assert_eq!(style.column_gap, px(28.0));
    assert_eq!(style.justify, Justify::Center);
    assert_eq!(style.align_items, Align::Center);
    assert_eq!(LayoutStyle::default().fill().width, percent(100.0));
    assert_eq!(LayoutStyle::default().hidden().display, Display::Hidden);
    assert_eq!(
        LayoutStyle::default().absolute().position,
        Position::Absolute
    );
    assert_eq!(LayoutStyle::default().wrap().wrap, Wrap::Yes);
    assert_eq!(LayoutStyle::default().width(auto()).width, auto());
}

#[test]
fn content_box_is_inside_padding_and_border() {
    let layout = Layout {
        rect: Rect::xywh(10.0, 20.0, 100.0, 50.0),
        padding: edges(1.0, 2.0, 3.0, 4.0),
        border: Edges::all(5.0),
    };
    assert_eq!(layout.content(), Rect::xywh(19.0, 26.0, 84.0, 36.0));
    let tiny = Layout {
        rect: Rect::xywh(0.0, 0.0, 4.0, 4.0),
        padding: Edges::all(10.0),
        border: Edges::default(),
    };
    assert_eq!(tiny.content().size, Size::ZERO, "never negative");
}

// ── L2: the pass ─────────────────────────────────────────────────────────────

#[test]
fn the_first_tick_lays_the_tree_out() {
    let mut app = app();
    let c = counter(&mut app);

    assert_eq!(tick(&mut app), LayoutDone::Recomputed);
    assert_eq!(rect(&app, c.me), Rect::xywh(0.0, 0.0, 480.0, 240.0));
    assert_eq!(rect(&app, c.dec), Rect::xywh(132.0, 88.0, 64.0, 64.0));
    assert_eq!(rect(&app, c.label), Rect::xywh(224.0, 88.0, 32.0, 64.0));
    assert_eq!(rect(&app, c.inc), Rect::xywh(284.0, 88.0, 64.0, 64.0));
}

#[test]
fn a_clean_tick_is_unchanged_and_moves_nothing() {
    let mut app = app();
    let c = counter(&mut app);
    tick(&mut app);
    let before = rect(&app, c.label);

    assert_eq!(tick(&mut app), LayoutDone::Unchanged);
    assert_eq!(rect(&app, c.label), before);
    assert_eq!(
        app.resource::<Done>().unwrap().0,
        [LayoutDone::Recomputed, LayoutDone::Unchanged],
        "signalled once per tick"
    );
}

#[test]
fn rects_are_absolute_through_nesting() {
    let mut app = app();
    let outer = app
        .spawn(
            app.root(),
            styled(
                LayoutStyle::default()
                    .absolute()
                    .size(px(200.0), px(200.0))
                    .padding_all(px(10.0)),
            ),
        )
        .unwrap();
    let inner = app
        .spawn(
            outer,
            styled(
                LayoutStyle::default()
                    .size(px(50.0), px(50.0))
                    .padding_all(px(5.0)),
            ),
        )
        .unwrap();
    let leaf = app.spawn(inner, measured(10.0, 10.0)).unwrap();
    tick(&mut app);

    assert_eq!(rect(&app, inner), Rect::xywh(10.0, 10.0, 50.0, 50.0));
    // A leaf in a row stretches to the row's content height by default.
    assert_eq!(rect(&app, leaf), Rect::xywh(15.0, 15.0, 10.0, 40.0));
    let layout = app.components::<Layout>().unwrap()[inner];
    assert_eq!(layout.padding, Edges::all(5.0));
    assert_eq!(layout.content(), Rect::xywh(15.0, 15.0, 40.0, 40.0));
}

#[test]
fn a_leaf_without_a_measure_is_zero_sized() {
    let mut app = app();
    let leaf = app
        .spawn(app.root(), styled(LayoutStyle::default()))
        .unwrap();
    tick(&mut app);
    assert_eq!(rect(&app, leaf).size, Size::ZERO);
}

#[test]
fn a_hidden_node_takes_no_space() {
    let mut app = app();
    let row = app
        .spawn(
            app.root(),
            styled(
                LayoutStyle::default()
                    .absolute()
                    .size(px(300.0), px(100.0))
                    .row(),
            ),
        )
        .unwrap();
    let a = app.spawn(row, measured(50.0, 50.0)).unwrap();
    let hidden = app
        .spawn(
            row,
            Styled(
                LayoutStyle::default().hidden(),
                Measure(Some(Size::new(50.0, 50.0))),
            ),
        )
        .unwrap();
    let b = app.spawn(row, measured(50.0, 50.0)).unwrap();
    tick(&mut app);

    assert_eq!(rect(&app, a).x(), 0.0);
    assert_eq!(rect(&app, hidden).size, Size::ZERO);
    assert_eq!(rect(&app, b).x(), 50.0, "b sits right after a");
}

#[test]
fn percent_is_of_the_parent() {
    let mut app = app();
    let outer = app
        .spawn(
            app.root(),
            styled(LayoutStyle::default().absolute().size(px(400.0), px(200.0))),
        )
        .unwrap();
    let half = app
        .spawn(
            outer,
            styled(LayoutStyle::default().size(percent(50.0), percent(25.0))),
        )
        .unwrap();
    tick(&mut app);
    assert_eq!(rect(&app, half).size, Size::new(200.0, 50.0));
}

#[test]
fn wrapping_rows_stack() {
    let mut app = app();
    let row = app
        .spawn(
            app.root(),
            styled(
                LayoutStyle::default()
                    .absolute()
                    .size(px(100.0), px(100.0))
                    .row()
                    .wrap(),
            ),
        )
        .unwrap();
    let ids: Vec<_> = (0..3)
        .map(|_| {
            app.spawn(row, styled(LayoutStyle::default().size(px(50.0), px(50.0))))
                .unwrap()
        })
        .collect();
    tick(&mut app);
    assert_eq!(rect(&app, ids[0]), Rect::xywh(0.0, 0.0, 50.0, 50.0));
    assert_eq!(rect(&app, ids[1]), Rect::xywh(50.0, 0.0, 50.0, 50.0));
    assert_eq!(
        rect(&app, ids[2]),
        Rect::xywh(0.0, 50.0, 50.0, 50.0),
        "align_content packs lines"
    );
}

// ── L3: dirtying ─────────────────────────────────────────────────────────────

#[test]
fn a_measure_write_relayouts() {
    let mut app = app();
    let c = counter(&mut app);
    tick(&mut app);

    app.components_mut::<Measure>().unwrap()[c.label] = Measure(Some(Size::new(48.0, 64.0)));
    assert_eq!(tick(&mut app), LayoutDone::Recomputed);
    assert_eq!(rect(&app, c.dec).origin, (124.0, 88.0).into());
    assert_eq!(rect(&app, c.label).origin, (216.0, 88.0).into());
    assert_eq!(rect(&app, c.inc).origin, (292.0, 88.0).into());
    assert_eq!(tick(&mut app), LayoutDone::Unchanged);
}

#[test]
fn a_style_write_relayouts() {
    let mut app = app();
    let c = counter(&mut app);
    tick(&mut app);

    let narrower = app.components::<LayoutStyle>().unwrap()[c.me]
        .clone()
        .width(px(400.0));
    set_style(&mut app, c.me, narrower);
    assert_eq!(tick(&mut app), LayoutDone::Recomputed);
    assert_eq!(rect(&app, c.me).width(), 400.0);
    assert_eq!(rect(&app, c.dec).x(), 92.0, "the row recentres");
    assert_eq!(tick(&mut app), LayoutDone::Unchanged);
}

#[test]
fn writing_an_equal_style_is_not_a_change() {
    let mut app = app();
    let c = counter(&mut app);
    tick(&mut app);
    let same = app.components::<LayoutStyle>().unwrap()[c.me].clone();
    set_style(&mut app, c.me, same);
    assert_eq!(tick(&mut app), LayoutDone::Unchanged);
}

#[test]
fn a_spawn_relayouts_once() {
    let mut app = app();
    let c = counter(&mut app);
    tick(&mut app);

    let fourth = app.spawn(c.me, measured(64.0, 64.0)).unwrap();
    assert_eq!(tick(&mut app), LayoutDone::Recomputed);
    assert_eq!(rect(&app, c.dec).x(), 86.0, "four items recentred");
    assert_eq!(rect(&app, fourth).origin, (330.0, 88.0).into());
    assert_eq!(tick(&mut app), LayoutDone::Unchanged);
}

#[test]
fn a_remove_relayouts_once() {
    let mut app = app();
    let c = counter(&mut app);
    let fourth = app.spawn(c.me, measured(64.0, 64.0)).unwrap();
    tick(&mut app);

    app.remove(fourth).unwrap();
    assert_eq!(tick(&mut app), LayoutDone::Recomputed);
    assert_eq!(rect(&app, c.dec).x(), 132.0, "back to three");
    assert_eq!(tick(&mut app), LayoutDone::Unchanged);
}

#[test]
fn a_deep_change_reaches_the_root() {
    let mut app = app();
    let outer = app
        .spawn(app.root(), styled(LayoutStyle::default().absolute().row()))
        .unwrap();
    let inner = app
        .spawn(outer, styled(LayoutStyle::default().row()))
        .unwrap();
    let leaf = app.spawn(inner, measured(10.0, 10.0)).unwrap();
    tick(&mut app);
    assert_eq!(rect(&app, outer).size, Size::new(10.0, 10.0));

    app.components_mut::<Measure>().unwrap()[leaf] = Measure(Some(Size::new(30.0, 10.0)));
    assert_eq!(tick(&mut app), LayoutDone::Recomputed);
    assert_eq!(
        rect(&app, outer).size,
        Size::new(30.0, 10.0),
        "the ancestors were re-measured"
    );
}

#[test]
fn a_reused_slot_starts_clean() {
    let mut app = app();
    let c = counter(&mut app);
    let extra = app.spawn(c.me, measured(64.0, 64.0)).unwrap();
    tick(&mut app);
    app.remove(extra).unwrap();
    tick(&mut app);

    // The freed slot is what the next spawn gets.
    let again = app.spawn(c.me, measured(20.0, 20.0)).unwrap();
    assert_eq!(again.id().component_index(), extra.id().component_index());
    assert_eq!(tick(&mut app), LayoutDone::Recomputed);
    assert_eq!(rect(&app, again).size, Size::new(20.0, 20.0));
}

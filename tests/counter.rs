//! The counter, headless. Layout, paint, window and the widgets are real;
//! the counter is local to this file. Nothing emits a press; the test does,
//! by hand, so the counter is stepped and asserted one div at a time.
//!
//! Every div shares the middle container, and their size is a derivation
//! from the count: the smallest power of two whose square holds it, so 1 is
//! the whole container, 2 to 4 are half, 5 to 16 are quarter, 17 to 64 are
//! eighth. A bar window above carries the count as a label.

use assets::{AtlasId, BakedFont, GlyphInfo};
use mecha_wayland::prelude::*;

// ── colours and font ─────────────────────────────────────────────────────────

const BG: Color = Color::rgb(0.10, 0.10, 0.12);
const BAR: Color = Color::rgb(0.16, 0.16, 0.20);
const RED: Color = Color::rgb(0.85, 0.25, 0.25);
const GREEN: Color = Color::rgb(0.25, 0.75, 0.35);
const ACCENT: Color = Color::rgb(0.30, 0.55, 0.95);

const fn glyphs() -> [GlyphInfo; 95] {
    let blank = GlyphInfo {
        x: 0.0,
        y: 0.0,
        w: 0.0,
        h: 0.0,
        bearing_x: 0.0,
        bearing_y: 0.0,
        advance: 8.0,
    };
    let mut g = [blank; 95];
    let mut i = 1;
    while i < 95 {
        g[i] = GlyphInfo {
            x: i as f32 * 10.0,
            y: 0.0,
            w: 6.0,
            h: 10.0,
            bearing_x: 1.0,
            bearing_y: -2.0,
            advance: 8.0,
        };
        i += 1;
    }
    g
}

static FONT: BakedFont = BakedFont {
    atlas_id: AtlasId(1),
    size: 12.0,
    line_height: 16.0,
    ascent: 12.0,
    glyphs: glyphs(),
};

// ── the counter ──────────────────────────────────────────────────────────────

/// The smallest power of two whose square holds `count`.
fn divisor(count: u32) -> u32 {
    let mut d = 1;
    while d * d < count {
        d *= 2;
    }
    d
}

/// Every div is the same square fraction of the middle container. Sizes
/// are border-box, so the border keeps the fraction exact and the divs
/// tile with no gap.
fn div_style(d: u32) -> LayoutStyle {
    let side = percent(100.0 / d as f32);
    LayoutStyle::default()
        .size(side, side)
        .border(Edges::all(px(2.0)))
}

#[derive(Clone, Copy)]
enum Press {
    Inc,
    Dec,
}
impl Event for Press {}

/// A handler cannot spawn or remove; it asks a system to.
struct AddDiv {
    counter: Handle<Counter>,
    parent: Handle<Div>,
    style: LayoutStyle,
}
impl Signal for AddDiv {}

struct RemoveDiv(Handle<Div>);
impl Signal for RemoveDiv {}

struct Counter {
    count: u32,
    win: Handle<Window>,
    bar: Handle<Window>,
    middle: Handle<Div>,
    divs: Vec<Handle<Div>>,
    label: Handle<Text>,
    inc: Handle<Div>,
    dec: Handle<Div>,
}
impl Widget for Counter {}

struct CounterBuilder;

impl WidgetBuild for CounterBuilder {
    type Widget = Counter;

    /// The counter's own node stays outside any window; its windows are
    /// children born with their style and background, and the content sits
    /// inside. The buttons are divs; the bar carries the label.
    fn spawn(self, me: Handle<Counter>, s: &mut Spawner<Counter>) -> Counter {
        let win = s.child(
            me,
            window()
                .title("counter")
                .layout(
                    LayoutStyle::default()
                        .size(px(480.0), px(240.0))
                        .row()
                        .gap(px(16.0))
                        .padding_all(px(16.0)),
                )
                .background(Quad::new(BG)),
        );
        let dec = s.child(win, div().size(px(48.0), px(48.0)).color(RED));
        let middle = s.child(
            win,
            div()
                .grow(1.0)
                .height(percent(100.0))
                .row()
                .wrap()
                .padding_all(px(2.0)),
        );
        let inc = s.child(win, div().size(px(48.0), px(48.0)).color(GREEN));

        let bar = s.child(
            me,
            window()
                .title("bar")
                .role(Role::Layer(Layer {
                    layer: LayerKind::Top,
                    anchor: Anchor::TOP | Anchor::LEFT | Anchor::RIGHT,
                    exclusive_zone: 32,
                    namespace: "counter-bar".into(),
                    ..Default::default()
                }))
                .layout(
                    LayoutStyle::default()
                        .size(px(480.0), px(32.0))
                        .padding_all(px(4.0))
                        .gap(px(8.0))
                        .align_items(Align::Center),
                )
                .background(Quad::new(BAR)),
        );
        s.child(
            bar,
            div().size(px(24.0), px(24.0)).color(ACCENT).radius(4.0),
        );
        let label = s.child(bar, text("0").font(&FONT));

        s.on(me, on_press);
        Counter {
            count: 0,
            win,
            bar,
            middle,
            divs: Vec::new(),
            label,
            inc,
            dec,
        }
    }
}

/// Inc: one more div. When the divisor changes, every existing div is
/// restyled first, so the new one joins a container already at the new
/// size. Dec: the last div goes. Either way the label follows the count.
fn on_press(ctx: &mut Context<Counter>, press: &Press) {
    let me = ctx.handle();
    let (count, middle, label, divs) = {
        let c = ctx.me();
        match press {
            Press::Inc => c.count += 1,
            Press::Dec if c.count > 0 => c.count -= 1,
            Press::Dec => return,
        }
        (c.count, c.middle, c.label, c.divs.clone())
    };
    ctx.emit(SetText(count.to_string()), &[label.id()]);
    match press {
        Press::Inc => {
            let d = divisor(count);
            if d != divisor(count - 1) {
                let ids: Vec<NodeId> = divs.iter().map(|h| h.id()).collect();
                ctx.emit(SetLayout(div_style(d)), &ids);
            }
            ctx.signal(AddDiv {
                counter: me,
                parent: middle,
                style: div_style(d),
            });
        }
        Press::Dec => {
            if let Some(last) = ctx.me().divs.pop() {
                ctx.signal(RemoveDiv(last));
            }
        }
    }
}

fn add_div(app: &mut App, e: &AddDiv) {
    let div = app
        .spawn(
            e.parent,
            div().layout(e.style.clone()).color(ACCENT).border(2.0, BG),
        )
        .unwrap();
    app.widget_mut::<Counter>(e.counter).unwrap().divs.push(div);
}

fn remove_div(app: &mut App, e: &RemoveDiv) {
    app.remove(e.0).unwrap();
}

// ── stub presentation ────────────────────────────────────────────────────────

/// How many times each window was asked for a frame since the last look.
#[derive(Default)]
struct Requests(Vec<(NodeId, u32)>);
impl Resource for Requests {}

fn on_request(app: &mut App, r: &RequestFrame) {
    let mut reqs = app.resource_mut::<Requests>().unwrap();
    match reqs.0.iter_mut().find(|(w, _)| *w == r.0) {
        Some((_, n)) => *n += 1,
        None => reqs.0.push((r.0, 1)),
    }
}

#[derive(Default)]
struct Closed(u32);
impl Resource for Closed {}

fn on_closed(app: &mut App, _: &AllWindowsClosed) {
    app.resource_mut::<Closed>().unwrap().0 += 1;
}

struct Asked(Vec<(NodeId, u32)>);

impl Asked {
    fn windows(&self) -> Vec<NodeId> {
        self.0.iter().map(|(w, _)| *w).collect()
    }
    fn count(&self, w: impl Into<NodeId>) -> u32 {
        let w = w.into();
        self.0
            .iter()
            .find(|(id, _)| *id == w)
            .map_or(0, |(_, n)| *n)
    }
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

fn asked(app: &mut App) -> Asked {
    Asked(std::mem::take(
        &mut app.resource_mut::<Requests>().unwrap().0,
    ))
}

// ── driving ──────────────────────────────────────────────────────────────────

/// The end of a batch.
fn tick(app: &mut App) {
    app.signal(Tick);
    app.flush();
}

/// A press is a batch of its own: the handler runs, the systems it asked
/// for run, and then the batch ends. A runner does the same: it drains what
/// its events queued before it signals `Tick`.
fn press(app: &mut App, counter: Handle<Counter>, press: Press) {
    app.emit(press, &[counter.id()]);
    app.flush();
    tick(app);
}

fn rect(app: &App, id: impl Into<NodeId>) -> Rect {
    app.components::<Layout>().unwrap()[id].rect
}

fn paint(app: &App, id: impl Into<NodeId>) -> Paint {
    app.components::<Paint>().unwrap()[id].clone()
}

fn label_glyphs(app: &App, label: Handle<Text>) -> usize {
    match paint(app, label) {
        Paint::Sprites(run) => run.len(),
        other => panic!("label paints {other:?}"),
    }
}

#[test]
fn headless_counter() {
    let mut app = App::new();
    app.add_module(LayoutModule)
        .add_module(PaintModule)
        .add_module(WindowModule)
        .insert_resource(Requests::default())
        .insert_resource(Closed::default())
        .system(add_div)
        .system(remove_div)
        .system(on_request)
        .system(on_closed);
    let counter = app.spawn(app.root(), CounterBuilder).unwrap();
    let (win, bar, inc, dec, middle, label) = {
        let c = app.widget::<Counter>(counter).unwrap();
        (c.win, c.bar, c.inc, c.dec, c.middle, c.label)
    };
    let root = app.root();

    // Windows are listed as they spawn; membership is the tree.
    assert_eq!(
        app.resource::<Windows>()
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        [win.id(), bar.id()]
    );
    assert_eq!(window_of(&app, middle.id()), Some(win.id()));
    assert_eq!(window_of(&app, label.id()), Some(bar.id()));
    assert_eq!(window_of(&app, counter.id()), None);

    // Every node carries the paint its builder gave it.
    assert_eq!(paint(&app, win), Paint::Quad(Quad::new(BG)));
    assert_eq!(paint(&app, bar), Paint::Quad(Quad::new(BAR)));
    assert_eq!(paint(&app, dec), Paint::Quad(Quad::new(RED)));
    assert_eq!(paint(&app, inc), Paint::Quad(Quad::new(GREEN)));
    assert_eq!(paint(&app, middle), Paint::Quad(Quad::default()));
    assert_eq!(paint(&app, root), Paint::None);
    assert_eq!(paint(&app, counter), Paint::None);
    assert_eq!(label_glyphs(&app, label), 1);

    // The spawn batch ends in a relayout, which asks every window for a
    // frame and nothing else.
    tick(&mut app);
    let a = asked(&mut app);
    assert_eq!(a.windows(), [win.id(), bar.id()]);
    assert!(a.count(win) >= 1 && a.count(bar) >= 1);
    assert_eq!(rect(&app, win), Rect::xywh(0.0, 0.0, 480.0, 240.0));
    assert_eq!(rect(&app, middle), Rect::xywh(80.0, 16.0, 320.0, 208.0));
    assert_eq!(rect(&app, bar), Rect::xywh(0.0, 0.0, 480.0, 32.0));
    assert_eq!(
        rect(&app, label),
        Rect::xywh(36.0, 8.0, 8.0, 16.0),
        "after the 24px square and the gap"
    );

    // A paint change asks its window once; a clean tick asks nothing.
    app.emit(SetQuad(Quad::new(RED).radius(8.0)), &[dec.id()]);
    app.flush();
    tick(&mut app);
    let a = asked(&mut app);
    assert_eq!(a.windows(), [win.id()]);
    assert_eq!(a.count(win), 1);
    tick(&mut app);
    assert!(asked(&mut app).is_empty());

    // The counter steps; the divisor changes exactly at 2, 5 and 17. Every
    // press moves the label, so the bar is asked too.
    for count in 1..=20u32 {
        press(&mut app, counter, Press::Inc);
        let a = asked(&mut app);
        assert!(
            a.count(win) >= 1 && a.count(bar) >= 1,
            "count {count}: {:?}",
            a.0
        );
        let c = app.widget::<Counter>(counter).unwrap();
        assert_eq!(c.count, count);
        assert_eq!(c.divs.len() as u32, count);
        let divs = c.divs.clone();
        assert_eq!(app.children(middle).unwrap().len(), count as usize);
        assert_eq!(label_glyphs(&app, label), count.to_string().len());
        // The container's 2px padding leaves 316px for the divs, so the
        // eighth is 39.5 and layout rounds it: half a pixel of slack.
        let expected = 316.0 / divisor(count) as f32;
        for div in &divs {
            let width = rect(&app, *div).width();
            assert!(
                (width - expected).abs() <= 0.5,
                "count {count}: width {width}, expected {expected}"
            );
            assert_eq!(
                paint(&app, *div),
                Paint::Quad(Quad::new(ACCENT).border(2.0, BG))
            );
        }
        let first = rect(&app, divs[0]);
        assert_eq!(
            first.origin,
            Point::new(82.0, 18.0),
            "tiles from the container's content corner"
        );
    }
    assert_eq!(divisor(1), 1);
    assert_eq!(divisor(2), 2);
    assert_eq!(divisor(4), 2);
    assert_eq!(divisor(5), 4);
    assert_eq!(divisor(16), 4);
    assert_eq!(divisor(17), 8);

    // Removing a div that was last in its wrap row moves no sibling, and
    // the window is still asked for a frame, by the relayout.
    press(&mut app, counter, Press::Dec);
    assert_eq!(app.widget::<Counter>(counter).unwrap().divs.len(), 19);
    assert_eq!(app.children(middle).unwrap().len(), 19);
    assert_eq!(label_glyphs(&app, label), 2);
    assert_eq!(asked(&mut app).count(win), 1);

    // A clean tick after all that asks nothing.
    tick(&mut app);
    assert!(asked(&mut app).is_empty());

    // The windows die with the counter; the last one closing is signalled.
    app.remove(counter).unwrap();
    tick(&mut app);
    assert!(app.resource::<Windows>().unwrap().is_empty());
    assert_eq!(app.resource::<Closed>().unwrap().0, 1);
    assert!(asked(&mut app).is_empty(), "nothing left to ask for");
}

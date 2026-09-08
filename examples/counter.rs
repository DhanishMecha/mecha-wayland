//! The counter, on the compositor: a toplevel with a `-` button, the count,
//! and a `+` button. Run under a Wayland compositor; close the window to
//! exit.
//!
//! Layout, paint, window, interactivity, presentation and rendering are the
//! real modules. What is local to this file is the counter and its button:
//! a button is a rounded quad with a label that turns a click into a
//! [`Press`] at the counter that owns it, and the counter answers a press by
//! rewriting its label. Nothing here names a protocol.

mod atlas {
    include!(concat!(env!("OUT_DIR"), "/counter_gen.rs"));
}

use mecha_wayland::assets::BakedFont;
use mecha_wayland::prelude::*;

const LABEL_FONT: &BakedFont = &atlas::COUNTER_FONT_INTER_28;
const COUNT_FONT: &BakedFont = &atlas::COUNTER_FONT_INTER_64;

const BG: Color = Color::from_rgb8(24, 24, 32);
const RED: Color = Color::from_rgb8(217, 64, 64);
const GREEN: Color = Color::from_rgb8(64, 191, 89);
const INK: Color = Color::from_rgb8(240, 240, 245);

// ── the button ───────────────────────────────────────────────────────────────

/// What a button asks its counter to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Press {
    Inc,
    Dec,
}
impl Event for Press {}

/// A rounded quad with a label. It knows the counter it belongs to, so a
/// click on it is a [`Press`] emitted there; the counter never has to know
/// which of its children was hit.
struct Button {
    counter: Handle<Counter>,
    press: Press,
}
impl Widget for Button {}

fn button(
    label: &'static str,
    color: Color,
    counter: Handle<Counter>,
    press: Press,
) -> ButtonBuilder {
    ButtonBuilder {
        label,
        color,
        counter,
        press,
    }
}

struct ButtonBuilder {
    label: &'static str,
    color: Color,
    counter: Handle<Counter>,
    press: Press,
}

impl WidgetBuild for ButtonBuilder {
    type Widget = Button;

    fn spawn(self, me: Handle<Button>, s: &mut Spawner<Button>) -> Button {
        s.set_component(LayoutStyle::default().size(px(72.0), px(72.0)).center());
        s.set_component(Paint::Quad(Quad::new(self.color).radius(16.0)));
        s.child(me, text(self.label).font(LABEL_FONT).color(INK));
        s.on(me, on_clicked);
        Button {
            counter: self.counter,
            press: self.press,
        }
    }
}

/// A left click anywhere in the button is a press at its counter.
fn on_clicked(ctx: &mut Context<Button>, click: &Clicked) {
    if click.button != MouseButton::Left {
        return;
    }
    let (counter, press) = {
        let me = ctx.me();
        (me.counter, me.press)
    };
    ctx.emit(press, &[counter.id()]);
}

// ── the counter ──────────────────────────────────────────────────────────────

struct Counter {
    count: u32,
    label: Handle<Text>,
}
impl Widget for Counter {}

struct CounterBuilder {
    start: u32,
}

impl WidgetBuild for CounterBuilder {
    type Widget = Counter;

    /// The counter's own node stays outside any window; its window is a
    /// child born with its style and background, and the row sits inside:
    /// a button, the count in a box wide enough that the buttons never
    /// move, a button.
    fn spawn(self, me: Handle<Counter>, s: &mut Spawner<Counter>) -> Counter {
        let win = s.child(
            me,
            window()
                .title("counter")
                .layout(
                    LayoutStyle::default()
                        .size(px(480.0), px(240.0))
                        .row()
                        .gap(px(28.0))
                        .center(),
                )
                .background(Quad::new(BG)),
        );
        s.child(win, button("-", RED, me, Press::Dec));
        let count_box = s.child(win, div().size(px(160.0), auto()).center());
        let label = s.child(
            count_box,
            text(self.start.to_string()).font(COUNT_FONT).color(INK),
        );
        let inc = s.child(win, button("+", GREEN, me, Press::Inc));
        s.on(me, on_press);
        Counter {
            count: self.start,
            label,
        }
    }
}

/// The count moves and the label follows; the label's new measure is a
/// relayout, and the relayout is a frame.
fn on_press(ctx: &mut Context<Counter>, press: &Press) {
    let (count, label) = {
        let me = ctx.me();
        match press {
            Press::Inc => me.count += 1,
            Press::Dec => me.count = me.count.saturating_sub(1),
        }
        (me.count, me.label)
    };
    ctx.emit(SetText(count.to_string()), &[label.id()]);
}

// ── main ─────────────────────────────────────────────────────────────────────

fn main() {
    let mut app = App::new();
    app
        .add_module(LayoutModule)
        .add_module(PaintModule)
        .add_module(WindowModule)
        .add_module(InteractivityModule)

        .add_module(RingModule::default())
        .add_module(WaylandModule)
        .add_module(PresentationModule)
        .add_module(RenderModule::new().atlas(&atlas::COUNTER));
    let root = app.root();
    app.spawn(root, CounterBuilder { start: 0 })
        .expect("the root is live");
    app.run();
    println!("counter: the window was closed");
}

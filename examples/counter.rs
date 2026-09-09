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
const INK: Color = Color::from_rgb8(240, 240, 245);

// ── the button ───────────────────────────────────────────────────────────────

/// What a button asks its counter to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Press {
    Inc,
    Dec,
    ToggleTheme,
}
impl Event for Press {}

/// A rounded quad with a label. It knows the counter it belongs to, so a
/// click on it is a [`Press`] emitted there; the counter never has to know
/// which of its children was hit.
struct Button {
    counter: Handle<Counter>,
    press: Press,
    color_role: ColorVariant,
}
impl Widget for Button {}

fn button(
    label: &'static str,
    color_role: ColorVariant,
    counter: Handle<Counter>,
    press: Press,
) -> ButtonBuilder {
    ButtonBuilder {
        label,
        color_role,
        counter,
        press,
    }
}

struct ButtonBuilder {
    label: &'static str,
    color_role: ColorVariant,
    counter: Handle<Counter>,
    press: Press,
}

impl WidgetBuild for ButtonBuilder {
    type Widget = Button;

    fn spawn(self, me: Handle<Button>, s: &mut Spawner<Button>) -> Button {
        let color = s
            .resource::<MechanixTheme>()
            .map(|theme| theme.color(self.color_role))
            .unwrap_or(RED);

        s.set_component(LayoutStyle::default().size(px(72.0), px(72.0)).center());
        s.set_component(Paint::Quad(Quad::new(color).radius(16.0)));
        s.child(me, text(self.label).font(LABEL_FONT).color(RED));
        // Re-resolve color when ThemeChanged broadcasts ApplyTheme
        s.on(me, |ctx: &mut Context<Button>, _: &ApplyTheme| {
            let role = ctx.me().color_role;
            if let Some(theme) = ctx.resource::<MechanixTheme>() {
                let new_color = theme.color(role);
                ctx.set_component(Paint::Quad(Quad::new(new_color).radius(16.0)));
            }
        });

        s.on(me, on_clicked);
        Button {
            counter: self.counter,
            press: self.press,
            color_role: self.color_role,
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
                        .size(px(560.0), px(240.0))
                        .row()
                        .gap(px(20.0))
                        .center(),
                )
                .background(Quad::new(BG)),
        );
        s.child(win, button("-", ColorVariant::Surface, me, Press::Dec));
        let count_box = s.child(win, div().size(px(140.0), auto()).center());
        let label = s.child(
            count_box,
            text(self.start.to_string()).font(COUNT_FONT).color(INK),
        );
        s.child(win, button("+", ColorVariant::Secondary, me, Press::Inc));
        s.child(
            win,
            button(
                "T",
                ColorVariant::SecondaryContainer,
                me,
                Press::ToggleTheme,
            ),
        );
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
    match press {
        Press::Inc | Press::Dec => {
            let (count, label) = {
                let me = ctx.me();
                match press {
                    Press::Inc => me.count += 1,
                    Press::Dec => me.count = me.count.saturating_sub(1),
                    _ => {}
                }
                (me.count.to_string(), me.label.id())
            };
            ctx.emit(SetText(count), &[label]);
        }
        Press::ToggleTheme => {
            let is_dark = ctx
                .resource::<MechanixTheme>()
                .map(|t| t.is_dark())
                .unwrap_or(true);

            if let Some(mut theme) = ctx.resource_mut::<MechanixTheme>() {
                *theme = if is_dark {
                    MechanixTheme::light()
                } else {
                    MechanixTheme::dark()
                };
            }
            ctx.signal(ThemeChanged);
        }
    }
}

// ── main ─────────────────────────────────────────────────────────────────────

fn main() {
    let mut app = App::new();
    app.add_module(MechanixTheme::dark())
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

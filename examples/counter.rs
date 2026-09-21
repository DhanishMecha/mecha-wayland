//! The counter, on the compositor: a toplevel with a `-` button, the count,
//! and a `+` button. Run under a Wayland compositor; close the window to
//! exit.
//!
//! Layout, paint, window, interactivity, presentation and rendering are the
//! real modules. The UI components use the `widgets` crate (`Button`, `Text`, `Div`).

mod atlas {
    include!(concat!(env!("OUT_DIR"), "/counter_gen.rs"));
}

use mecha_wayland::assets::BakedFont;
use mecha_wayland::prelude::*;

const COUNT_FONT: &BakedFont = &atlas::COUNTER_FONT_INTER_64;

// ── events ───────────────────────────────────────────────────────────────────

/// What a button asks its counter to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Press {
    Inc,
    Dec,
    ToggleTheme,
}
impl Event for Press {}

// ── button handlers ──────────────────────────────────────────────────────────

fn on_dec_click(ctx: &mut Context<Button>, click: &Clicked) {
    if click.button == MouseButton::Left {
        ctx.emit_all(Press::Dec);
    }
}

fn on_inc_click(ctx: &mut Context<Button>, click: &Clicked) {
    if click.button == MouseButton::Left {
        ctx.emit_all(Press::Inc);
    }
}

fn on_toggle_click(ctx: &mut Context<Button>, click: &Clicked) {
    if click.button == MouseButton::Left {
        ctx.emit_all(Press::ToggleTheme);
    }
}

// ── the counter ──────────────────────────────────────────────────────────────

struct Counter {
    count: u32,
    label: Handle<Text>,
    container: Handle<Div>,
}
impl Widget for Counter {}

struct CounterBuilder {
    start: u32,
}

impl WidgetBuild for CounterBuilder {
    type Widget = Counter;

    /// The counter's own node stays outside any window; its window is a
    /// child born with its style and background, and the row sits inside:
    /// a decrement button, the count in a box, an increment button, and a theme toggle button.
    fn spawn(self, me: Handle<Counter>, s: &mut Spawner<Counter>) -> Counter {
        let bg = s.color(ThemeColor::Background);
        let win = s.child(
            me,
            window()
                .title("counter")
                .layout(LayoutStyle::default().size(px(560.0), px(240.0)).column())
                .background(Quad::new(bg)),
        );

        let container = s.child(
            win,
            div()
                .fill()
                .row()
                .gap(px(20.0))
                .center()
                .paint(Quad::new(bg)),
        );

        let btn_dec = s.child(container, button("dec").size(ButtonSize::MEDIUM));
        s.on(btn_dec, on_dec_click);

        let count_box = s.child(container, div().size(px(140.0), auto()).center());
        let label = s.child(count_box, text(self.start.to_string()).font(COUNT_FONT));

        let btn_inc = s.child(container, button("inc").size(ButtonSize::MEDIUM));
        s.on(btn_inc, on_inc_click);

        let btn_toggle = s.child(container, button_outlined("Theme").size(ButtonSize::MEDIUM));
        s.on(btn_toggle, on_toggle_click);

        s.on(me, on_press);
        s.on(me, on_theme_changed);
        Counter {
            count: self.start,
            label,
            container,
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
            let next_theme = match ctx.theme_mode() {
                ThemeMode::Dark => MechanixTheme::light(),
                ThemeMode::Light => MechanixTheme::dark(),
            };
            ctx.set_theme(next_theme);
        }
    }
}

fn on_theme_changed(ctx: &mut Context<Counter>, _: &theme::ApplyTheme) {
    let bg = ctx.color(ThemeColor::Background);
    let container_id = ctx.me().container.id();
    ctx.emit(SetQuad(Quad::new(bg)), &[container_id]);
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
        .add_module(
            RenderModule::new()
                .atlas(&WIDGET_FONTS)
                .atlas(&atlas::COUNTER),
        );
    let root = app.root();
    app.spawn(root, CounterBuilder { start: 0 })
        .expect("the root is live");
    app.run();
    println!("counter: the window was closed");
}

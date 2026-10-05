//! Demonstrates the `Button` widget from the `widgets` crate.
//!
//! Run under a Wayland session:
//!   `cargo run --example button_demo`
//!
//! The window shows:
//!   - A counter label controlled by +/- buttons (Filled variant)
//!   - A "Reset" button (Tonal variant)
//!   - A "Toggle Theme" button (Outlined variant)
//!   - An "Inactive" disabled button
//!   - A "Text Button" (Text variant)

use mecha_wayland::prelude::*;

// ── Shell ─────────────────────────────────────────────────────────────────────

struct Shell;
struct ShellBuilder {
    root: NodeId,
    font: FontId,
}
impl Build for ShellBuilder {
    type Widget = Shell;
}

impl Widget for Shell {
    type Builder = ShellBuilder;
    fn build(b: ShellBuilder, _me: Handle<Self>, s: &mut Spawner<'_, Self>) -> Self {
        let win = s.spawn(
            b.root,
            window()
                .title("Button Demo")
                .clear(Color::rgb(0.08, 0.08, 0.10))
                .layout(LayoutStyle::default().center().size(px(520.0), px(700.0))),
        );
        s.on::<CloseRequested>(win, |ctx, _| ctx.signal(Stop));
        s.spawn(win, DemoBuilder { font: b.font });
        Shell
    }
}

// ── Demo ──────────────────────────────────────────────────────────────────────

struct Demo {
    count: i32,
}

struct DemoBuilder {
    font: FontId,
}
impl Build for DemoBuilder {
    type Widget = Demo;
}

impl Widget for Demo {
    type Builder = DemoBuilder;

    fn build(b: DemoBuilder, me: Handle<Demo>, s: &mut Spawner<'_, Demo>) -> Demo {
        let font = b.font;

        // Outer column, fills the window, centres everything.
        *s.component_mut::<LayoutStyle>(me).unwrap() = LayoutStyle::default()
            .column()
            .center()
            .fill()
            .gap(px(16.0));

        // ── Counter label ─────────────────────────────────────────────────────
        let count_label = s.spawn(
            me,
            text(font, "0")
                .size(48)
                .color(s.color(ColorRole::OnSurface)),
        );

        // Keep label colour in sync with theme changes.
        s.on_theme(me, move |ctx| {
            let fg = ctx.color(ColorRole::OnSurface);
            if let Some(mut lctx) = ctx.at(count_label) {
                lctx.set_color(fg);
            }
        });

        // ── +/- row (Filled buttons) ──────────────────────────────────────────
        let row = s.spawn(
            me,
            div().style(LayoutStyle::default().row().center().gap(px(12.0))),
        );

        let minus = s.spawn(row, button().child(text(font, " − ")).border_radius(10.0));
        let plus = s.spawn(row, button().child(text(font, " + ")));

        s.on::<Clicked>(minus, move |ctx, _| {
            ctx.me().count -= 1;
            let v = ctx.me().count;
            if let Some(mut lctx) = ctx.at(count_label) {
                lctx.set_text(v.to_string());
            }
        });

        s.on::<Clicked>(plus, move |ctx, _| {
            ctx.me().count += 1;
            let v = ctx.me().count;
            if let Some(mut lctx) = ctx.at(count_label) {
                lctx.set_text(v.to_string());
            }
        });

        // ── Reset ─────────────────────────────────────────────────────────────
        let reset = s.spawn(row, button().child(text(font, "Reset")));

        s.on::<Clicked>(reset, move |ctx, _| {
            ctx.me().count = 0;
            if let Some(mut lctx) = ctx.at(count_label) {
                lctx.set_text("0");
            }
        });

        // ── Outlined Mechanix button ──────────────────────────────────────────
        s.spawn(
            me,
            mechanix_button()
                .variant(MechanixButtonVariant::Outlined)
                .child(text(font, "Outlined Button"))
                .on_click(|_ctx| {}),
        );

        // ── Disabled Mechanix button ──────────────────────────────────────────
        s.spawn(
            me,
            mechanix_button()
                .variant(MechanixButtonVariant::Filled)
                .child(text(font, "Disabled Button"))
                .disabled(true),
        );

        // ── Small / Large size demo ───────────────────────────────────────────
        let size_row = s.spawn(
            me,
            div().style(LayoutStyle::default().row().center().gap(px(8.0))),
        );
        s.spawn(
            size_row,
            button()
                .child(text(font, "Small"))
                .height(px(32.0))
                .padding(Insets::symmetric(px(16.0), px(0.0)))
                .border_radius(10.0)
                .on_click(|_ctx| {}),
        );
        s.spawn(
            size_row,
            button()
                .child(text(font, "Large"))
                .height(px(72.0))
                .padding(Insets::symmetric(px(48.0), px(0.0)))
                .on_click(|_ctx| {}),
        );

        // ── Theme toggle ──────────────────────────────────────────────────────
        s.spawn(
            me,
            button()
                .child(text(font, "Toggle Theme"))
                .border(1.0, Color::rgb(0.5, 0.5, 0.5))
                .on_click(|ctx| {
                    let next = match ctx.theme().mode() {
                        ThemeMode::Dark => MechanixTheme::light(),
                        ThemeMode::Light => MechanixTheme::dark(),
                    };
                    ctx.set_theme(next);
                }),
        );

        Demo { count: 0 }
    }
}

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    let mut app = App::new();
    app.add_module(LayoutModule)
        .add_module(PaintModule)
        .add_module(WindowModule)
        .add_module(InteractivityModule)
        .add_module(RenderModule::default())
        .insert_resource(Atlas::new());

    app.add_module(MechanixTheme::dark());

    app.add_module(RingModule::default())
        .add_module(
            WaylandModule::new()
                .bind::<WlCompositor>()
                .bind::<ZwpLinuxDmabufV1>()
                .bind::<XdgWmBase>()
                .bind::<WlSeat>(),
        )
        .add_module(PresentationModule {
            app_id: "mecha.button_demo".into(),
            budget: Budget::default(),
        });

    let font = app
        .resource_mut::<Atlas>()
        .add_font(include_bytes!(
            "../crates/atlas/tests/fixtures/Inter-Regular.ttf"
        ))
        .expect("Inter loads");
    app.insert_resource(FontBook::new(font));

    let root = app.root();
    app.spawn(root, ShellBuilder { root, font });
    app.run();
}

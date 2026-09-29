//! A `+`/`-` pair that bump a label each click. Run under a Wayland
//! session: `cargo run --example counter`.

use mecha_wayland::prelude::*;

/// A label between a `-` and a `+`; each click moves the count by one.
struct Counter {
    count: i32,
}

impl Counter {
    fn step(&mut self, delta: i32) -> i32 {
        self.count += delta;
        self.count
    }
}

fn counter(font: FontId) -> CounterBuilder {
    CounterBuilder { font }
}

struct CounterBuilder {
    font: FontId,
}

impl Build for CounterBuilder {
    type Widget = Counter;
}

impl Widget for Counter {
    type Builder = CounterBuilder;
    fn build(b: CounterBuilder, me: Handle<Self>, s: &mut Spawner<'_, Self>) -> Self {
        *s.component_mut::<LayoutStyle>(me).unwrap() =
            LayoutStyle::default().column().center().gap(px(16.0));

        let label = s.spawn(
            me,
            text(b.font, "0")
                .size(32)
                .color(s.color(ColorRole::OnSurface)),
        );
        let row = s.spawn(
            me,
            div().style(LayoutStyle::default().row().center().gap(px(12.0))),
        );
        s.spawn(
            row,
            button(b.font, " - ")
                .border_radius(10.)
                .on_click(move |ctx| step_counter(ctx, me, label, -1)),
        );

        s.spawn(
            row,
            button(b.font, " + ").on_click(move |ctx| step_counter(ctx, me, label, 1)),
        );

        Counter { count: 0 }
    }
}

fn step_counter(
    ctx: &mut Context<'_, Button>,
    me: Handle<Counter>,
    label: Handle<Text>,
    delta: i32,
) {
    if let Some(mut cctx) = ctx.at(me) {
        let count = cctx.me().step(delta);
        if let Some(mut lctx) = cctx.at(label) {
            lctx.set_text(count.to_string());
        }
    }
}

/// Spawns the window and the counter; stops the app on close.
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
                .title("counter")
                .clear(Color::rgb(0.12, 0.12, 0.14))
                .layout(LayoutStyle::default().center().size(px(260.0), px(180.0))),
        );
        s.on::<CloseRequested>(win, |ctx, _| ctx.signal(Stop));
        s.spawn(win, counter(b.font));
        Shell
    }
}

fn main() {
    let mut app = App::new();
    app.add_module(LayoutModule)
        .add_module(PaintModule)
        .add_module(WindowModule)
        .add_module(InteractivityModule)
        .add_module(RenderModule::default())
        .add_module(MechanixTheme::dark())
        .insert_resource(Atlas::new());
    app.add_module(RingModule::default())
        .add_module(
            WaylandModule::new()
                .bind::<WlCompositor>()
                .bind::<ZwpLinuxDmabufV1>()
                .bind::<XdgWmBase>()
                .bind::<WlSeat>(),
        )
        .add_module(PresentationModule {
            app_id: "mecha.counter".into(),
            budget: Budget::default(),
        });

    let font = app
        .resource_mut::<Atlas>()
        .add_font(include_bytes!(
            "../crates/atlas/tests/fixtures/Inter-Regular.ttf"
        ))
        .expect("Inter loads");

    let root = app.root();
    app.spawn(root, ShellBuilder { root, font });
    app.run();
}

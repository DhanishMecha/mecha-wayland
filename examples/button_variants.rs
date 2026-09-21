//! The button variants example on the compositor: a window displaying all
//! button types (Filled and Outlined), sizes (Extra Small, Small, Medium, Large, Extra Large),
//! interaction states, custom styling, and live theme switching.
//! Run under a Wayland compositor; close the window to exit.
use mecha_wayland::prelude::*;

// ── events ───────────────────────────────────────────────────────────────────

/// Action emitted when UI buttons are clicked.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Action {
    ToggleTheme,
    ButtonClicked(&'static str),
}
impl Event for Action {}

// ── button handlers ──────────────────────────────────────────────────────────

fn on_toggle_click(ctx: &mut Context<Button>, click: &Clicked) {
    if click.button == MouseButton::Left {
        ctx.emit_all(Action::ToggleTheme);
    }
}

fn on_filled_click(ctx: &mut Context<Button>, click: &Clicked) {
    if click.button == MouseButton::Left {
        ctx.emit_all(Action::ButtonClicked("Filled Button"));
    }
}

fn on_outlined_click(ctx: &mut Context<Button>, click: &Clicked) {
    if click.button == MouseButton::Left {
        ctx.emit_all(Action::ButtonClicked("Outlined Button"));
    }
}

fn on_xs_click(ctx: &mut Context<Button>, click: &Clicked) {
    if click.button == MouseButton::Left {
        ctx.emit_all(Action::ButtonClicked("Extra Small Button"));
    }
}

fn on_sm_click(ctx: &mut Context<Button>, click: &Clicked) {
    if click.button == MouseButton::Left {
        ctx.emit_all(Action::ButtonClicked("Small Button"));
    }
}

fn on_md_click(ctx: &mut Context<Button>, click: &Clicked) {
    if click.button == MouseButton::Left {
        ctx.emit_all(Action::ButtonClicked("Medium Button"));
    }
}

fn on_lg_click(ctx: &mut Context<Button>, click: &Clicked) {
    if click.button == MouseButton::Left {
        ctx.emit_all(Action::ButtonClicked("Large Button"));
    }
}

fn on_xl_click(ctx: &mut Context<Button>, click: &Clicked) {
    if click.button == MouseButton::Left {
        ctx.emit_all(Action::ButtonClicked("Extra Large Button"));
    }
}

fn on_acc_filled_click(ctx: &mut Context<Button>, click: &Clicked) {
    if click.button == MouseButton::Left {
        ctx.emit_all(Action::ButtonClicked("Accented Filled Button"));
    }
}

fn on_acc_outlined_click(ctx: &mut Context<Button>, click: &Clicked) {
    if click.button == MouseButton::Left {
        ctx.emit_all(Action::ButtonClicked("Accented Outlined Button"));
    }
}

fn on_custom_click(ctx: &mut Context<Button>, click: &Clicked) {
    if click.button == MouseButton::Left {
        ctx.emit_all(Action::ButtonClicked("Custom Styled Button"));
    }
}

// ── main widget ──────────────────────────────────────────────────────────────

struct ButtonVariantsViewer {
    container: Handle<Div>,
    card_divs: Vec<Handle<Div>>,
    status_label: Handle<Text>,
}
impl Widget for ButtonVariantsViewer {}

struct ButtonVariantsViewerBuilder;

impl WidgetBuild for ButtonVariantsViewerBuilder {
    type Widget = ButtonVariantsViewer;

    fn spawn(
        self,
        me: Handle<ButtonVariantsViewer>,
        s: &mut Spawner<ButtonVariantsViewer>,
    ) -> ButtonVariantsViewer {
        let bg = s.color(ThemeColor::Background);
        let card_bg = s.color(ThemeColor::SurfaceContainer);

        let win = s.child(
            me,
            window()
                .title("button_variants")
                .layout(LayoutStyle::default().size(px(1000.0), px(1200.0)).column())
                .background(Quad::new(bg)),
        );

        let container = s.child(
            win,
            div()
                .fill()
                .column()
                .padding_all(px(24.0))
                .gap(px(16.0))
                .color(bg),
        );

        // Header bar with title, subtitle, and Theme Toggle button
        let header_bar = s.child(
            container,
            div()
                .row()
                .justify(Justify::SpaceBetween)
                .align_items(Align::Center)
                .width(percent(100.0)),
        );

        let title_box = s.child(header_bar, div().column().gap(px(4.0)));
        s.child(
            title_box,
            text("Mechanix Button Variants").variant(TextVariant::HeadlineMedium),
        );
        s.child(
            title_box,
            text("Interactive preview of Material 3 button types, sizes, states, and themes")
                .variant(TextVariant::BodyMedium),
        );

        let btn_toggle = s.child(
            header_bar,
            button_outlined("Toggle Theme").size(ButtonSize::MEDIUM),
        );
        s.on(btn_toggle, on_toggle_click);

        // Status Bar
        let status_card = s.child(
            container,
            div()
                .row()
                .padding_all(px(12.0))
                .gap(px(8.0))
                .radius(8.0)
                .color(s.color(ThemeColor::SurfaceContainerHigh))
                .width(percent(100.0)),
        );

        s.child(
            status_card,
            text("Last Action: ")
                .variant(TextVariant::LabelLarge)
                .color(ThemeColor::Primary),
        );

        let status_label = s.child(
            status_card,
            text("No button clicked yet")
                .variant(TextVariant::LabelLarge)
                .color(ThemeColor::OnSurface),
        );

        let mut card_divs = Vec::new();
        card_divs.push(status_card);

        // 1. Material 3 Button Types Card
        let card1 = s.child(
            container,
            div()
                .column()
                .padding_all(px(16.0))
                .gap(px(12.0))
                .radius(12.0)
                .color(card_bg)
                .width(percent(100.0)),
        );
        card_divs.push(card1);

        s.child(
            card1,
            text("1. Button Types")
                .variant(TextVariant::TitleSmall)
                .color(ThemeColor::Primary),
        );

        let types_row = s.child(
            card1,
            div().row().gap(px(12.0)).align_items(Align::Center).wrap(),
        );

        let btn_filled = s.child(types_row, button("Filled Button"));
        s.on(btn_filled, on_filled_click);

        let btn_out = s.child(types_row, button_outlined("Outlined Button"));
        s.on(btn_out, on_outlined_click);

        // 2. Button Sizes Card
        let card2 = s.child(
            container,
            div()
                .column()
                .padding_all(px(16.0))
                .gap(px(12.0))
                .radius(12.0)
                .color(card_bg)
                .width(percent(100.0)),
        );
        card_divs.push(card2);

        s.child(
            card2,
            text("2. Button Sizes (Presets)")
                .variant(TextVariant::TitleSmall)
                .color(ThemeColor::Primary),
        );

        let sizes_row = s.child(
            card2,
            div().row().gap(px(12.0)).align_items(Align::Center).wrap(),
        );

        let b_xs = s.child(
            sizes_row,
            button("Extra Small").size(ButtonSize::EXTRA_SMALL),
        );
        s.on(b_xs, on_xs_click);

        let b_sm = s.child(sizes_row, button("Small").size(ButtonSize::SMALL));
        s.on(b_sm, on_sm_click);

        let b_md = s.child(sizes_row, button("Medium").size(ButtonSize::MEDIUM));
        s.on(b_md, on_md_click);

        let b_lg = s.child(sizes_row, button("Large").size(ButtonSize::LARGE));
        s.on(b_lg, on_lg_click);

        let b_xl = s.child(
            sizes_row,
            button("Extra Large").size(ButtonSize::EXTRA_LARGE),
        );
        s.on(b_xl, on_xl_click);

        // 3. Interaction States Card
        let card3 = s.child(
            container,
            div()
                .column()
                .padding_all(px(16.0))
                .gap(px(12.0))
                .radius(12.0)
                .color(card_bg)
                .width(percent(100.0)),
        );
        card_divs.push(card3);

        s.child(
            card3,
            text("3. Interaction States")
                .variant(TextVariant::TitleSmall)
                .color(ThemeColor::Primary),
        );

        let states_row = s.child(
            card3,
            div().row().gap(px(12.0)).align_items(Align::Center).wrap(),
        );

        let b_enabled = s.child(states_row, button("Enabled").state(WidgetState::Enabled));
        s.on(b_enabled, on_filled_click);

        let b_hovered = s.child(states_row, button("Hovered").state(WidgetState::Hovered));
        s.on(b_hovered, on_filled_click);

        let b_focused = s.child(states_row, button("Focused").state(WidgetState::Focused));
        s.on(b_focused, on_filled_click);

        let b_pressed = s.child(states_row, button("Pressed").state(WidgetState::Pressed));
        s.on(b_pressed, on_filled_click);

        let _b_disabled = s.child(states_row, button("Disabled").state(WidgetState::Disabled));

        // 4. Accented & Custom Styling Card
        let card4 = s.child(
            container,
            div()
                .column()
                .padding_all(px(16.0))
                .gap(px(12.0))
                .radius(12.0)
                .color(card_bg)
                .width(percent(100.0)),
        );
        card_divs.push(card4);

        s.child(
            card4,
            text("4. Accented & Custom Styling")
                .variant(TextVariant::TitleSmall)
                .color(ThemeColor::Primary),
        );

        let custom_row = s.child(
            card4,
            div().row().gap(px(12.0)).align_items(Align::Center).wrap(),
        );

        let b_acc_filled = s.child(custom_row, button("Accented Filled").accented());
        s.on(b_acc_filled, on_acc_filled_click);

        let b_acc_out = s.child(custom_row, button_outlined("Accented Outlined").accented());
        s.on(b_acc_out, on_acc_outlined_click);

        let b_custom = s.child(
            custom_row,
            button("Custom Colors & Radius")
                .background_color(Color::from_rgb8(138, 43, 226))
                .border_radius(20.0),
        );
        s.on(b_custom, on_custom_click);

        s.on(me, on_action);
        s.on(me, on_theme_changed);

        ButtonVariantsViewer {
            container,
            card_divs,
            status_label,
        }
    }
}

fn on_action(ctx: &mut Context<ButtonVariantsViewer>, action: &Action) {
    match action {
        Action::ToggleTheme => {
            let next_theme = match ctx.theme_mode() {
                ThemeMode::Dark => MechanixTheme::light(),
                ThemeMode::Light => MechanixTheme::dark(),
            };
            ctx.set_theme(next_theme);
        }
        Action::ButtonClicked(name) => {
            let label_id = ctx.me().status_label.id();
            let msg = format!("Clicked: {}", name);
            ctx.emit(SetText(msg), &[label_id]);
        }
    }
}

fn on_theme_changed(ctx: &mut Context<ButtonVariantsViewer>, _: &theme::ApplyTheme) {
    let bg = ctx.color(ThemeColor::Background);
    let card_bg = ctx.color(ThemeColor::SurfaceContainer);

    let container_id = ctx.me().container.id();
    ctx.emit(SetQuad(Quad::new(bg)), &[container_id]);

    let card_ids: Vec<_> = ctx.me().card_divs.iter().map(|h| h.id()).collect();
    for &id in &card_ids {
        ctx.emit(SetQuad(Quad::new(card_bg).radius(12.0)), &[id]);
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
        .add_module(RenderModule::new().atlas(&WIDGET_FONTS));

    let root = app.root();
    app.spawn(root, ButtonVariantsViewerBuilder)
        .expect("the root is live");
    app.run();
    println!("button_variants: the window was closed");
}

//! The text variants example on the compositor: a window displaying all 15
//! Mechanix typography text variants (Display, Headline, Title, Body, Label)
//! in both standard and emphasised styles, along with style overrides (color roles, weight,
//! letter spacing), and interactive theme toggling. All colors are dynamically derived
//! from the active [`MechanixTheme`] color scheme.
//! Run under a Wayland compositor; close the window to exit.

use mecha_wayland::prelude::*;

// ── events ───────────────────────────────────────────────────────────────────

/// Action emitted when UI buttons are clicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    ToggleTheme,
}
impl Event for Action {}

// ── button handlers ──────────────────────────────────────────────────────────

fn on_toggle_click(ctx: &mut Context<Button>, click: &Clicked) {
    if click.button == MouseButton::Left {
        ctx.emit_all(Action::ToggleTheme);
    }
}

// ── main widget ──────────────────────────────────────────────────────────────

struct TextVariantsViewer {
    container: Handle<Div>,
    card_divs: Vec<Handle<Div>>,
}
impl Widget for TextVariantsViewer {}

struct TextVariantsViewerBuilder;

impl WidgetBuild for TextVariantsViewerBuilder {
    type Widget = TextVariantsViewer;

    fn spawn(
        self,
        me: Handle<TextVariantsViewer>,
        s: &mut Spawner<TextVariantsViewer>,
    ) -> TextVariantsViewer {
        let bg = s.color(ThemeColor::Background);
        let card_bg = s.color(ThemeColor::SurfaceContainer);

        let win = s.child(
            me,
            window()
                .title("text_variants")
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
            text("Mechanix Typography Variants").variant(TextVariant::HeadlineMedium),
        );
        s.child(
            title_box,
            text("Interactive preview of all 15 typography roles with live theme switching")
                .variant(TextVariant::BodyMedium),
        );

        let btn_toggle = s.child(
            header_bar,
            button_outlined("Toggle Theme").size(ButtonSize::MEDIUM),
        );
        s.on(btn_toggle, on_toggle_click);

        let mut card_divs = Vec::new();

        // 1. Display Category Card
        let card1 = s.child(
            container,
            div()
                .column()
                .padding_all(px(16.0))
                .gap(px(8.0))
                .radius(12.0)
                .color(card_bg)
                .width(percent(100.0)),
        );
        card_divs.push(card1);

        s.child(
            card1,
            text("Display Variants (Hero Titles)")
                .variant(TextVariant::TitleSmall)
                .color(ThemeColor::Primary),
        );
        s.child(
            card1,
            text("Display Large (57px)").variant(TextVariant::DisplayLarge),
        );
        s.child(
            card1,
            text("Display Medium (45px)").variant(TextVariant::DisplayMedium),
        );
        s.child(
            card1,
            text("Display Small (36px)").variant(TextVariant::DisplaySmall),
        );

        // 2. Headline Category Card
        let card2 = s.child(
            container,
            div()
                .column()
                .padding_all(px(16.0))
                .gap(px(8.0))
                .radius(12.0)
                .color(card_bg)
                .width(percent(100.0)),
        );
        card_divs.push(card2);

        s.child(
            card2,
            text("Headline Variants (High-emphasis Headers)")
                .variant(TextVariant::TitleSmall)
                .color(ThemeColor::Primary),
        );
        s.child(
            card2,
            text("Headline Large (32px)").variant(TextVariant::HeadlineLarge),
        );
        s.child(
            card2,
            text("Headline Medium (28px)").variant(TextVariant::HeadlineMedium),
        );
        s.child(
            card2,
            text("Headline Small (24px)").variant(TextVariant::HeadlineSmall),
        );

        // 3. Title Category Card
        let card3 = s.child(
            container,
            div()
                .column()
                .padding_all(px(16.0))
                .gap(px(8.0))
                .radius(12.0)
                .color(card_bg)
                .width(percent(100.0)),
        );
        card_divs.push(card3);

        s.child(
            card3,
            text("Title Variants (Medium-emphasis Subheadings)")
                .variant(TextVariant::TitleSmall)
                .color(ThemeColor::Primary),
        );
        s.child(
            card3,
            text("Title Large (22px)").variant(TextVariant::TitleLarge),
        );
        s.child(
            card3,
            text("Title Medium (16px)").variant(TextVariant::TitleMedium),
        );
        s.child(
            card3,
            text("Title Small (14px)").variant(TextVariant::TitleSmall),
        );

        // 4. Body Category Card
        let card4 = s.child(
            container,
            div()
                .column()
                .padding_all(px(16.0))
                .gap(px(8.0))
                .radius(12.0)
                .color(card_bg)
                .width(percent(100.0)),
        );
        card_divs.push(card4);

        s.child(
            card4,
            text("Body Variants (Passages & Paragraphs)")
                .variant(TextVariant::TitleSmall)
                .color(ThemeColor::Primary),
        );
        s.child(
            card4,
            text("Body Large (16px) — Used for primary long-form body text and readable prose.")
                .variant(TextVariant::BodyLarge),
        );
        s.child(
            card4,
            text("Body Medium (14px) — Default paragraph text style for standard UI components.")
                .variant(TextVariant::BodyMedium),
        );
        s.child(
            card4,
            text(
                "Body Small (12px) — Used for secondary explanations, fine print, and annotations.",
            )
            .variant(TextVariant::BodySmall),
        );

        // 5. Label Category Card
        let card5 = s.child(
            container,
            div()
                .column()
                .padding_all(px(16.0))
                .gap(px(8.0))
                .radius(12.0)
                .color(card_bg)
                .width(percent(100.0)),
        );
        card_divs.push(card5);

        s.child(
            card5,
            text("Label Variants (Buttons & Metadata)")
                .variant(TextVariant::TitleSmall)
                .color(ThemeColor::Primary),
        );
        s.child(
            card5,
            text("LABEL LARGE (14px) — Action buttons and prominent labels")
                .variant(TextVariant::LabelLarge),
        );
        s.child(
            card5,
            text("LABEL MEDIUM (12px) — Category tags, field headers, and tabs")
                .variant(TextVariant::LabelMedium),
        );
        s.child(
            card5,
            text("LABEL SMALL (11px) — Status badges, timestamps, and subtle hints")
                .variant(TextVariant::LabelSmall),
        );

        // 6. Style Overrides Card
        // let card6 = s.child(
        //     container,
        //     div()
        //         .column()
        //         .padding_all(px(16.0))
        //         .gap(px(8.0))
        //         .radius(12.0)
        //         .color(card_bg)
        //         .width(percent(100.0)),
        // );
        // card_divs.push(card6);

        // s.child(
        //     card6,
        //     text("Style Overrides & Modifiers")
        //         .variant(TextVariant::TitleSmall)
        //         .color(ThemeColor::Primary),
        // );
        // s.child(
        //     card6,
        //     text("Emphasised Weight Variant (.emphasised(true))")
        //         .variant(TextVariant::BodyLarge)
        //         .emphasised(true),
        // );
        // s.child(
        //     card6,
        //     text("Semantic Color Role (.color(ThemeColor::Tertiary))")
        //         .variant(TextVariant::BodyMedium)
        //         .color(ThemeColor::Tertiary),
        // );
        // s.child(
        //     card6,
        //     text("Explicit Color Token (.color(ThemeColor::Error))")
        //         .variant(TextVariant::BodyMedium)
        //         .color(ThemeColor::Error),
        // );
        // s.child(
        //     card6,
        //     text("Explicit Font Weight Override (.weight(FontWeight::Bold))")
        //         .variant(TextVariant::BodyMedium)
        //         .weight(FontWeight::Bold),
        // );
        // s.child(
        //     card6,
        //     text("CUSTOM LETTER SPACING (.letter_spacing(4.0))")
        //         .variant(TextVariant::LabelMedium)
        //         .letter_spacing(4.0),
        // );

        s.on(me, on_action);
        s.on(me, on_theme_changed);

        TextVariantsViewer {
            container,
            card_divs,
        }
    }
}

fn on_action(ctx: &mut Context<TextVariantsViewer>, action: &Action) {
    match action {
        Action::ToggleTheme => {
            let next_theme = match ctx.theme_mode() {
                ThemeMode::Dark => MechanixTheme::light(),
                ThemeMode::Light => MechanixTheme::dark(),
            };
            ctx.set_theme(next_theme);
        }
    }
}

fn on_theme_changed(ctx: &mut Context<TextVariantsViewer>, _: &theme::ApplyTheme) {
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
    app.spawn(root, TextVariantsViewerBuilder)
        .expect("the root is live");
    app.run();
    println!("text_variants: the window was closed");
}

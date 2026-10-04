//! Builder and runtime tests for [`Button`].
//!
//! Verifies:
//! 1. Last call wins between explicit overrides and variant/style resets.
//! 2. variant() preserves size and shape.
//! 3. Disabled handling fades colors with opacity.
//! 4. Layout derivation and builder methods.
//! 5. State transitions (including Exit from Pressed).
//! 6. Runtime setters on `ButtonContextExt`.
//! 7. Font resolution via `FontBook` and font override.

use app::prelude::*;
use atlas::prelude::*;
use geometry::{Color, Point};
use interactivity::{Clicked, ContactId, Enter, Exit, InteractivityModule, Press, Release};
use layout::prelude::*;
use mechanix_widgets::prelude::*;
use paint::prelude::*;
use theme::prelude::*;
use widgets::prelude::div;
use window::prelude::*;

const INTER: &[u8] = include_bytes!("../../../crates/atlas/tests/fixtures/Inter-Regular.ttf");

fn app_with_theme(mode: ThemeMode) -> (App, FontId) {
    let mut app = App::new();
    app.add_module(LayoutModule)
        .add_module(PaintModule)
        .add_module(WindowModule)
        .add_module(InteractivityModule);
    app.insert_resource(Atlas::new());
    let theme = match mode {
        ThemeMode::Dark => MechanixTheme::dark(),
        ThemeMode::Light => MechanixTheme::light(),
    };
    app.add_module(theme);
    let font = app
        .resource_mut::<Atlas>()
        .add_font(INTER)
        .expect("Inter loads");
    app.insert_resource(FontBook::new(font));
    (app, font)
}

fn root(app: &mut App, w: f32, h: f32) -> NodeId {
    app.spawn_with(
        app.root(),
        div().style(LayoutStyle::default().size(px(w), px(h))),
        (LayoutRoot(true),),
    )
    .id()
}

fn resolved(app: &mut App, r: NodeId, b: ButtonBuilder) -> ResolvedButtonStyle {
    let handle: Handle<Button> = app.spawn(r, b);
    app.tick();
    let w = app.widget::<Button>(handle).unwrap();
    w.tokens.resolve(app.theme(), w.state, &w.overrides)
}

// ── Hierarchy & Precedence Tests ─────────────────────────────────────────────

#[test]
fn test_hierarchy_last_write_wins() {
    let (mut app, _f) = app_with_theme(ThemeMode::Dark);
    let r = root(&mut app, 200.0, 100.0);
    let red = Color::rgb(1.0, 0.0, 0.0);

    // 1. .background_color(red).variant(Outlined) -> variant defined last wins (transparent bg)
    let s1 = resolved(
        &mut app,
        r,
        button("A")
            .background_color(red)
            .variant(ButtonVariant::Outlined),
    );
    assert_eq!(s1.background_color, Color::TRANSPARENT);

    // 2. .variant(Outlined).border_color(red) -> setter after variant wins (red border)
    let s2 = resolved(
        &mut app,
        r,
        button("A")
            .variant(ButtonVariant::Outlined)
            .border_color(red),
    );
    assert_eq!(s2.border_color, red);

    // 3. .size(LARGE).variant(Outlined) -> variant retains LARGE size
    let b3 = button("A")
        .size(ButtonSize::LARGE)
        .variant(ButtonVariant::Outlined);
    let h3 = app.spawn(r, b3);
    app.tick();
    assert_eq!(
        app.widget::<Button>(h3).unwrap().tokens.size,
        ButtonSize::LARGE
    );
}

#[test]
fn test_style_resets_overrides() {
    let (mut app, _f) = app_with_theme(ThemeMode::Dark);
    let r = root(&mut app, 200.0, 100.0);
    let red = Color::rgb(1.0, 0.0, 0.0);

    let s = resolved(
        &mut app,
        r,
        button("A")
            .background_color(red)
            .style(ButtonStyle::outlined()),
    );
    // style() clears explicit background override
    assert_eq!(s.background_color, Color::TRANSPARENT);
}

#[test]
fn test_accented_mode() {
    let (mut app, _f) = app_with_theme(ThemeMode::Dark);
    let r = root(&mut app, 200.0, 100.0);

    let s = resolved(
        &mut app,
        r,
        button("A")
            .variant(ButtonVariant::Filled)
            .accented(),
    );
    // Accented filled uses Primary background
    assert_eq!(s.background_color, app.theme().colors.primary);
}

#[test]
fn test_layout_style_override() {
    let (mut app, _f) = app_with_theme(ThemeMode::Dark);
    let r = root(&mut app, 200.0, 100.0);

    // .layout(...) after .width(...) resets explicit width override
    let btn_reset = app.spawn(
        r,
        button("Reset Width")
            .width(px(120.0))
            .layout(LayoutStyle::default().width(px(200.0))),
    );
    app.tick();
    let div_reset = app.widget::<Button>(btn_reset).unwrap().div_handle.unwrap();
    let layout_reset = app.component::<LayoutStyle>(div_reset).unwrap();
    assert_eq!(layout_reset.width, Val::Px(200.0));

    // .width(...) after .layout(...) overrides the layout width
    let btn = app.spawn(
        r,
        button("Custom Layout")
            .size(ButtonSize::LARGE)
            .layout(LayoutStyle::default().fill())
            .width(px(120.0)),
    );
    app.tick();
    let div = app.widget::<Button>(btn).unwrap().div_handle.unwrap();
    let layout = app.component::<LayoutStyle>(div).unwrap();
    assert_eq!(layout.width, Val::Px(120.0));
}

// ── Disabled State Tests ─────────────────────────────────────────────────────

#[test]
fn test_disabled_state_fades_overrides() {
    let theme = MechanixTheme::dark();
    let red = Color::rgba(1.0, 0.0, 0.0, 1.0);
    let overrides = ButtonOverrides {
        background_color: Some(red),
        border_color: Some(red),
        content_color: Some(red),
        ..Default::default()
    };

    let style = ButtonStyle::filled();
    let resolved = style.resolve(&theme, WidgetState::Disabled, &overrides);

    // Opacity for disabled filled container is 0.10, label is 0.38, border is 0.0
    assert!((resolved.background_color.a - (1.0 * style.disabled_background_opacity)).abs() < 1e-4);
    assert!((resolved.content_color.a - (1.0 * style.disabled_label_opacity)).abs() < 1e-4);
    assert!((resolved.border_color.a - (1.0 * style.disabled_border_opacity)).abs() < 1e-4);
}

#[test]
fn test_disabled_outlined_button_matches_theme_spec() {
    let theme = MechanixTheme::dark();
    let style = ButtonStyle::outlined();
    let overrides = ButtonOverrides::default();
    let resolved = style.resolve(&theme, WidgetState::Disabled, &overrides);

    // Outlined disabled: transparent bg, 0.38 label alpha, 0.10 border alpha
    assert_eq!(resolved.background_color, Color::TRANSPARENT);
    assert!((resolved.content_color.a - 0.38).abs() < 1e-4);
    assert!((resolved.border_color.a - 0.10).abs() < 1e-4);
}

// ── Transition Tests ─────────────────────────────────────────────────────────

#[test]
fn test_state_transitions() {
    let (mut app, _f) = app_with_theme(ThemeMode::Dark);
    let r = root(&mut app, 200.0, 100.0);
    let btn: Handle<Button> = app.spawn(r, button("Test"));
    app.tick();
    assert_eq!(
        app.widget::<Button>(btn).unwrap().state,
        WidgetState::Enabled
    );

    let enter = Enter {
        contact: ContactId::Mouse,
        position: Point::new(0.0, 0.0),
    };
    let exit = Exit {
        contact: ContactId::Mouse,
        position: Point::new(0.0, 0.0),
    };
    let press = Press {
        contact: ContactId::Mouse,
        position: Point::new(0.0, 0.0),
    };
    let release = Release {
        contact: ContactId::Mouse,
        position: Point::new(0.0, 0.0),
    };

    // Enter -> Hovered
    app.emit(enter, btn);
    app.tick();
    assert_eq!(
        app.widget::<Button>(btn).unwrap().state,
        WidgetState::Hovered
    );

    // Press -> Pressed
    app.emit(press, btn);
    app.tick();
    assert_eq!(
        app.widget::<Button>(btn).unwrap().state,
        WidgetState::Pressed
    );

    // Exit while pressed -> Enabled (must not remain stuck in Pressed!)
    app.emit(exit, btn);
    app.tick();
    assert_eq!(
        app.widget::<Button>(btn).unwrap().state,
        WidgetState::Enabled
    );

    // Press again while enabled
    app.emit(press, btn);
    app.tick();
    assert_eq!(
        app.widget::<Button>(btn).unwrap().state,
        WidgetState::Pressed
    );

    // Release -> Hovered
    app.emit(release, btn);
    app.tick();
    assert_eq!(
        app.widget::<Button>(btn).unwrap().state,
        WidgetState::Hovered
    );

    // SetButtonState event directly
    app.emit(SetButtonState(WidgetState::Disabled), btn);
    app.tick();
    assert_eq!(
        app.widget::<Button>(btn).unwrap().state,
        WidgetState::Disabled
    );
}

#[test]
fn test_runtime_setters() {
    let (mut app, _f) = app_with_theme(ThemeMode::Dark);
    let r = root(&mut app, 200.0, 100.0);
    let btn: Handle<Button> = app.spawn(
        r,
        button("Initial")
            .size(ButtonSize::LARGE)
            .background_color(Color::rgb(1.0, 0.0, 0.0))
            .on_click(|ctx| {
                ctx.set_variant(ButtonVariant::Outlined);
                ctx.set_label("Updated");
            }),
    );
    app.tick();

    // Trigger on_click via Clicked event
    app.emit(
        Clicked {
            contact: ContactId::Mouse,
            position: Point::new(0.0, 0.0),
        },
        btn,
    );
    app.tick();

    let w = app.widget::<Button>(btn).unwrap();
    // Overrides should be cleared by set_variant
    assert_eq!(w.overrides.background_color, None);
    // Size should be preserved
    assert_eq!(w.tokens.size, ButtonSize::LARGE);
    // Background should now be transparent
    let div_handle = w.div_handle.unwrap();
    match app.component::<Paint>(div_handle).unwrap() {
        Paint::Quad(q) => assert_eq!(q.color, Color::TRANSPARENT),
        other => panic!("expected Quad, got {other:?}"),
    }

    // Check label was updated
    let label_handle = w.label_handle.unwrap();
    assert_eq!(
        app.widget::<widgets::Text>(label_handle).unwrap().text(),
        "Updated"
    );
}

#[test]
fn test_theme_switch_updates_button() {
    let (mut app, _f) = app_with_theme(ThemeMode::Dark);
    let r = root(&mut app, 200.0, 100.0);
    let btn: Handle<Button> = app.spawn(r, button("Theme"));
    app.tick();
    let div_handle = app.widget::<Button>(btn).unwrap().div_handle.unwrap();
    let dark_paint = app.component::<Paint>(div_handle).cloned();

    app.set_theme(MechanixTheme::light());
    app.tick();
    let light_paint = app.component::<Paint>(div_handle).cloned();

    assert_ne!(dark_paint, light_paint);
}

#[test]
fn test_font_override() {
    let (mut app, font) = app_with_theme(ThemeMode::Dark);
    let r = root(&mut app, 200.0, 100.0);
    let btn: Handle<Button> = app.spawn(r, button("Explicit Font").font(font));
    app.tick();
    let w = app.widget::<Button>(btn).unwrap();
    assert_eq!(w.font, Some(font));
}

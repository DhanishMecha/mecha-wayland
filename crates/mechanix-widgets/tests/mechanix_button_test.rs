//! Tests for [`MechanixButton`]: Material 3 / Mechanix design token button
//! wrapping the raw [`Button`] widget.
//!
//! Verifies:
//! 1. Filled variant defaults and styling.
//! 2. Outlined variant defaults and styling.
//! 3. Accented mode for filled and outlined.
//! 4. Content forwarding via `.child(...)`.
//! 5. Interaction state transitions (Hovered, Pressed) update the state layer overlay.
//! 6. Disabled state ignores clicks and applies opacity.
//! 7. Theme reactivity updates button styling on theme switch.
//! 8. Runtime setters (set_variant, set_size, set_disabled, resolved_style).

use app::prelude::*;
use atlas::prelude::*;
use geometry::{Color, Insets, Point};
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

// ── 1. Filled variant defaults ────────────────────────────────────────────────

#[test]
fn test_mechanix_button_filled_default() {
    let (mut app, _) = app_with_theme(ThemeMode::Dark);
    let r = root(&mut app, 200.0, 100.0);

    let btn: Handle<MechanixButton> = app.spawn(r, mechanix_button());
    app.tick();

    let mb = app.widget::<MechanixButton>(btn).unwrap();
    assert_eq!(mb.variant(), MechanixButtonVariant::Filled);
    assert_eq!(mb.state(), WidgetState::Enabled);

    // Inner Button handle exists
    let raw_btn_h = mb.button_handle().unwrap();
    let raw_btn = app.widget::<Button>(raw_btn_h).unwrap();
    let div_h = raw_btn.div_handle().unwrap();

    let paint = app.component::<Paint>(div_h).unwrap();
    if let Paint::Quad(quad) = paint {
        let expected_bg = ColorRole::SecondaryFixedDim.resolve(&app.theme().colors);
        assert_eq!(quad.color, expected_bg);
        assert_eq!(quad.border, Insets::all(0.0));
    } else {
        panic!("expected Paint::Quad");
    }
}

// ── 2. Outlined variant defaults ──────────────────────────────────────────────

#[test]
fn test_mechanix_button_outlined() {
    let (mut app, _) = app_with_theme(ThemeMode::Dark);
    let r = root(&mut app, 200.0, 100.0);

    let btn: Handle<MechanixButton> = app.spawn(
        r,
        mechanix_button().variant(MechanixButtonVariant::Outlined),
    );
    app.tick();

    let mb = app.widget::<MechanixButton>(btn).unwrap();
    assert_eq!(mb.variant(), MechanixButtonVariant::Outlined);

    let raw_btn_h = mb.button_handle().unwrap();
    let raw_btn = app.widget::<Button>(raw_btn_h).unwrap();
    let div_h = raw_btn.div_handle().unwrap();

    let paint = app.component::<Paint>(div_h).unwrap();
    if let Paint::Quad(quad) = paint {
        assert_eq!(quad.color, Color::TRANSPARENT);
        let expected_border = ColorRole::Outline.resolve(&app.theme().colors);
        assert_eq!(quad.border_color, expected_border);
        assert_eq!(quad.border, Insets::all(2.0));
    } else {
        panic!("expected Paint::Quad");
    }
}

// ── 3. Accented mode ──────────────────────────────────────────────────────────

#[test]
fn test_mechanix_button_accented() {
    let (mut app, _) = app_with_theme(ThemeMode::Dark);
    let r = root(&mut app, 200.0, 100.0);

    // Accented filled
    let filled_btn: Handle<MechanixButton> = app.spawn(
        r,
        mechanix_button()
            .variant(MechanixButtonVariant::Filled)
            .accented(),
    );
    app.tick();

    let mb = app.widget::<MechanixButton>(filled_btn).unwrap();
    let raw_btn = app.widget::<Button>(mb.button_handle().unwrap()).unwrap();
    let paint = app.component::<Paint>(raw_btn.div_handle().unwrap()).unwrap();
    if let Paint::Quad(quad) = paint {
        let expected_bg = ColorRole::Primary.resolve(&app.theme().colors);
        assert_eq!(quad.color, expected_bg);
    } else {
        panic!("expected Paint::Quad");
    }

    // Accented outlined
    let outlined_btn: Handle<MechanixButton> = app.spawn(
        r,
        mechanix_button()
            .variant(MechanixButtonVariant::Outlined)
            .accented(),
    );
    app.tick();

    let mb_out = app.widget::<MechanixButton>(outlined_btn).unwrap();
    let raw_btn_out = app.widget::<Button>(mb_out.button_handle().unwrap()).unwrap();
    let paint_out = app.component::<Paint>(raw_btn_out.div_handle().unwrap()).unwrap();
    if let Paint::Quad(quad) = paint_out {
        let expected_border = ColorRole::Primary.resolve(&app.theme().colors);
        assert_eq!(quad.border_color, expected_border);
    } else {
        panic!("expected Paint::Quad");
    }
}

// ── 4. Child forwarding ───────────────────────────────────────────────────────

#[test]
fn test_mechanix_button_with_child() {
    let (mut app, font) = app_with_theme(ThemeMode::Dark);
    let r = root(&mut app, 200.0, 100.0);

    use widgets::prelude::text as native_text;
    let btn: Handle<MechanixButton> = app.spawn(
        r,
        mechanix_button().child(
            native_text(font, "Click Me").size(14).color(Color::WHITE),
        ),
    );
    app.tick();

    let mb = app.widget::<MechanixButton>(btn).unwrap();
    let raw_btn = app.widget::<Button>(mb.button_handle().unwrap()).unwrap();
    let div_h = raw_btn.div_handle().unwrap();
    let children = app.children(div_h).unwrap_or(&[]);
    assert_eq!(children.len(), 1, "child must be parented under inner div");
}

// ── 5. State transitions and state layer overlay ──────────────────────────────

#[test]
fn test_mechanix_button_state_transitions() {
    let (mut app, _) = app_with_theme(ThemeMode::Dark);
    let r = root(&mut app, 200.0, 100.0);

    let btn: Handle<MechanixButton> = app.spawn(r, mechanix_button());
    app.tick();

    let mb = app.widget::<MechanixButton>(btn).unwrap();
    let raw_btn_h = mb.button_handle().unwrap();
    let div_h = app.widget::<Button>(raw_btn_h).unwrap().div_handle().unwrap();

    let initial_color = match app.component::<Paint>(div_h).unwrap() {
        Paint::Quad(q) => q.color,
        _ => panic!(),
    };

    // Hover on inner button
    app.emit(Enter { contact: ContactId::Mouse, position: Point::new(0.0, 0.0) }, raw_btn_h);
    app.tick();

    let hovered_color = match app.component::<Paint>(div_h).unwrap() {
        Paint::Quad(q) => q.color,
        _ => panic!(),
    };
    assert_ne!(hovered_color, initial_color);

    // Press on inner button
    app.emit(Press { contact: ContactId::Mouse, position: Point::new(0.0, 0.0) }, raw_btn_h);
    app.tick();

    let pressed_color = match app.component::<Paint>(div_h).unwrap() {
        Paint::Quad(q) => q.color,
        _ => panic!(),
    };
    assert_ne!(pressed_color, hovered_color);

    // Release back to Hovered
    app.emit(Release { contact: ContactId::Mouse, position: Point::new(0.0, 0.0) }, raw_btn_h);
    app.tick();
    let released_color = match app.component::<Paint>(div_h).unwrap() {
        Paint::Quad(q) => q.color,
        _ => panic!(),
    };
    assert_eq!(released_color, hovered_color);

    // Exit back to Enabled
    app.emit(Exit { contact: ContactId::Mouse, position: Point::new(0.0, 0.0) }, raw_btn_h);
    app.tick();
    let exit_color = match app.component::<Paint>(div_h).unwrap() {
        Paint::Quad(q) => q.color,
        _ => panic!(),
    };
    assert_eq!(exit_color, initial_color);
}

// ── 6. Click handling and disabled ────────────────────────────────────

#[test]
fn test_mechanix_button_click_and_disabled() {
    let (mut app, _) = app_with_theme(ThemeMode::Dark);
    let r = root(&mut app, 200.0, 100.0);

    let count = std::rc::Rc::new(std::cell::Cell::new(0));
    let c = count.clone();

    let btn: Handle<MechanixButton> = app.spawn(
        r,
        mechanix_button().on_click(move |_ctx| {
            c.set(c.get() + 1);
        }),
    );
    app.tick();

    app.emit(
        Clicked { contact: ContactId::Mouse, position: Point::new(0.0, 0.0) },
        btn,
    );
    app.tick();
    assert_eq!(count.get(), 1);

    // Now test a disabled button
    let disabled_clicked = std::rc::Rc::new(std::cell::Cell::new(0));
    let dc = disabled_clicked.clone();
    let disabled_btn: Handle<MechanixButton> = app.spawn(
        r,
        mechanix_button()
            .disabled(true)
            .on_click(move |_ctx| {
                dc.set(dc.get() + 1);
            }),
    );
    app.tick();

    app.emit(
        Clicked { contact: ContactId::Mouse, position: Point::new(0.0, 0.0) },
        disabled_btn,
    );
    app.tick();
    assert_eq!(disabled_clicked.get(), 0, "click must not fire when disabled");
}

// ── 7. Theme reactivity ───────────────────────────────────────────────────────

#[test]
fn test_mechanix_button_theme_switch() {
    let (mut app, _) = app_with_theme(ThemeMode::Dark);
    let r = root(&mut app, 200.0, 100.0);

    let btn: Handle<MechanixButton> = app.spawn(r, mechanix_button());
    app.tick();

    let mb = app.widget::<MechanixButton>(btn).unwrap();
    let raw_btn = app.widget::<Button>(mb.button_handle().unwrap()).unwrap();
    let div_h = raw_btn.div_handle().unwrap();

    let dark_paint = app.component::<Paint>(div_h).cloned();

    app.set_theme(MechanixTheme::light());
    app.tick();

    let light_paint = app.component::<Paint>(div_h).cloned();
    assert_ne!(dark_paint, light_paint, "paint must update on theme switch");
}

// ── 8. Runtime setters ────────────────────────────────────────────────────────

#[test]
fn test_mechanix_button_runtime_setters() {
    let (mut app, _) = app_with_theme(ThemeMode::Dark);
    let r = root(&mut app, 200.0, 100.0);

    let btn: Handle<MechanixButton> = app.spawn(
        r,
        mechanix_button().on_click(|ctx| {
            ctx.set_variant(MechanixButtonVariant::Outlined);
            ctx.set_size(MechanixButtonSize::LARGE);
        }),
    );
    app.tick();

    app.emit(
        Clicked { contact: ContactId::Mouse, position: Point::new(0.0, 0.0) },
        btn,
    );
    app.tick();

    let mb = app.widget::<MechanixButton>(btn).unwrap();
    assert_eq!(mb.variant(), MechanixButtonVariant::Outlined);
    assert_eq!(mb.size(), MechanixButtonSize::LARGE);

    let raw_btn = app.widget::<Button>(mb.button_handle().unwrap()).unwrap();
    let div_h = raw_btn.div_handle().unwrap();
    let paint = app.component::<Paint>(div_h).unwrap();
    if let Paint::Quad(quad) = paint {
        assert_eq!(quad.color, Color::TRANSPARENT);
        assert_eq!(quad.border, Insets::all(2.0));
    } else {
        panic!("expected Paint::Quad");
    }
}

//! Tests for the raw generic [`Button`] container widget.
//!
//! Verifies:
//! 1. Button can be built with no child content (content-agnostic).
//! 2. Button can accept arbitrary child content (text, inner container, composite row).
//! 3. All interaction state transitions work (Enabled → Hovered → Pressed → Hovered → Enabled).
//! 4. Disabled button ignores click callbacks.
//! 5. Enabled button fires click callbacks.
//! 6. Runtime context setters (set_disabled, set_background, set_border, set_radius, set_padding).
//! 7. Raw builder properties (background, border, radius, padding, width, height, layout).
//! 8. SetButtonState event directly sets button state.
//! 9. Interaction behavior is independent of child content.

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

// ── 1. Content-agnostic: Button with no child ─────────────────────────────────

#[test]
fn test_button_with_no_child_content() {
    let (mut app, _) = app_with_theme(ThemeMode::Dark);
    let r = root(&mut app, 200.0, 100.0);

    let btn: Handle<Button> = app.spawn(r, button());
    app.tick();

    let w = app.widget::<Button>(btn).unwrap();
    assert!(w.div_handle.is_some(), "div container must exist");
    assert_eq!(w.state, WidgetState::Enabled);
}

// ── 2. Button accepts arbitrary child content ─────────────────────────────────

#[test]
fn test_button_with_text_child() {
    let (mut app, font) = app_with_theme(ThemeMode::Dark);
    let r = root(&mut app, 200.0, 100.0);

    use widgets::prelude::text as native_text;
    let btn: Handle<Button> = app.spawn(
        r,
        button().child(
            native_text(font, "Hello").size(14).color(Color::WHITE),
        ),
    );
    app.tick();

    let w = app.widget::<Button>(btn).unwrap();
    let div_h = w.div_handle.unwrap();
    let children = app.children(div_h).unwrap_or(&[]);
    assert_eq!(children.len(), 1, "text child should be parented under div");
}

#[test]
fn test_button_with_container_child() {
    let (mut app, _) = app_with_theme(ThemeMode::Dark);
    let r = root(&mut app, 200.0, 100.0);

    let btn: Handle<Button> = app.spawn(
        r,
        button().child(
            div().style(LayoutStyle::default().size(px(24.0), px(24.0))),
        ),
    );
    app.tick();

    let w = app.widget::<Button>(btn).unwrap();
    let div_h = w.div_handle.unwrap();
    let children = app.children(div_h).unwrap_or(&[]);
    assert_eq!(children.len(), 1, "container child should be parented under div");
}

#[test]
fn test_button_with_composite_child() {
    let (mut app, font) = app_with_theme(ThemeMode::Dark);
    let r = root(&mut app, 300.0, 100.0);

    use widgets::prelude::text as native_text;
    let btn: Handle<Button> = app.spawn(
        r,
        button().child_fn(move |s, container| {
            let row = s.spawn(
                container,
                div().style(LayoutStyle::default().row().center().gap(px(8.0))),
            );
            s.spawn(
                row,
                div().style(LayoutStyle::default().size(px(20.0), px(20.0))),
            );
            s.spawn(
                row,
                native_text(font, "Label").size(14).color(Color::WHITE),
            );
            row.id()
        }),
    );
    app.tick();

    let w = app.widget::<Button>(btn).unwrap();
    let div_h = w.div_handle.unwrap();
    let children = app.children(div_h).unwrap_or(&[]);
    assert_eq!(children.len(), 1, "composite row should be parented under div");
}

// ── 3. State transitions ──────────────────────────────────────────────────────

#[test]
fn test_state_transitions() {
    let (mut app, _) = app_with_theme(ThemeMode::Dark);
    let r = root(&mut app, 200.0, 100.0);
    let btn: Handle<Button> = app.spawn(r, button());
    app.tick();

    assert_eq!(app.widget::<Button>(btn).unwrap().state, WidgetState::Enabled);

    // Enter -> Hovered
    app.emit(Enter { contact: ContactId::Mouse, position: Point::new(0.0, 0.0) }, btn);
    app.tick();
    assert_eq!(app.widget::<Button>(btn).unwrap().state, WidgetState::Hovered);

    // Press -> Pressed
    app.emit(Press { contact: ContactId::Mouse, position: Point::new(0.0, 0.0) }, btn);
    app.tick();
    assert_eq!(app.widget::<Button>(btn).unwrap().state, WidgetState::Pressed);

    // Release -> Hovered
    app.emit(Release { contact: ContactId::Mouse, position: Point::new(0.0, 0.0) }, btn);
    app.tick();
    assert_eq!(app.widget::<Button>(btn).unwrap().state, WidgetState::Hovered);

    // Exit -> Enabled
    app.emit(Exit { contact: ContactId::Mouse, position: Point::new(0.0, 0.0) }, btn);
    app.tick();
    assert_eq!(app.widget::<Button>(btn).unwrap().state, WidgetState::Enabled);
}

// ── 4. Click handling & disabled behaviour ────────────────────────────────────

#[test]
fn test_enabled_button_fires_click() {
    let (mut app, _) = app_with_theme(ThemeMode::Dark);
    let r = root(&mut app, 200.0, 100.0);

    let clicked = std::rc::Rc::new(std::cell::Cell::new(0));
    let c = clicked.clone();
    let btn: Handle<Button> = app.spawn(
        r,
        button().on_click(move |_ctx| {
            c.set(c.get() + 1);
        }),
    );
    app.tick();

    app.emit(
        Clicked { contact: ContactId::Mouse, position: Point::new(0.0, 0.0) },
        btn,
    );
    app.tick();
    assert_eq!(clicked.get(), 1);
}

#[test]
fn test_disabled_button_ignores_click() {
    let (mut app, _) = app_with_theme(ThemeMode::Dark);
    let r = root(&mut app, 200.0, 100.0);

    let clicked = std::rc::Rc::new(std::cell::Cell::new(0));
    let c = clicked.clone();
    let btn: Handle<Button> = app.spawn(
        r,
        button()
            .disabled(true)
            .on_click(move |_ctx| {
                c.set(c.get() + 1);
            }),
    );
    app.tick();

    app.emit(
        Clicked { contact: ContactId::Mouse, position: Point::new(0.0, 0.0) },
        btn,
    );
    app.tick();
    assert_eq!(clicked.get(), 0, "disabled button must not fire click");
}

// ── 5. Runtime context setters ────────────────────────────────────────────────

#[test]
fn test_runtime_context_setters() {
    let (mut app, _) = app_with_theme(ThemeMode::Dark);
    let r = root(&mut app, 200.0, 100.0);

    let new_bg = Color::rgb(0.9, 0.1, 0.2);
    let new_border = Color::rgb(0.1, 0.9, 0.2);

    let btn: Handle<Button> = app.spawn(
        r,
        button().on_click(move |ctx| {
            ctx.set_disabled(true);
            ctx.set_background(new_bg);
            ctx.set_border(3.0, new_border);
            ctx.set_radius(8.0);
            ctx.set_container_padding(Insets::all(px(10.0)));
        }),
    );
    app.tick();

    // Trigger on_click
    app.emit(
        Clicked { contact: ContactId::Mouse, position: Point::new(0.0, 0.0) },
        btn,
    );
    app.tick();

    let w = app.widget::<Button>(btn).unwrap();
    assert_eq!(w.state(), WidgetState::Disabled);
    assert_eq!(w.background(), Some(new_bg));
    assert_eq!(w.border_color(), Some(new_border));
    assert_eq!(w.border_thickness(), 3.0);
    assert_eq!(w.border_radius(), 8.0);
    assert_eq!(w.padding(), Some(Insets::all(px(10.0))));

    let div_h = w.div_handle().unwrap();
    let paint = app.component::<Paint>(div_h).unwrap();
    if let Paint::Quad(quad) = paint {
        assert_eq!(quad.color, new_bg);
        assert_eq!(quad.border_color, new_border);
        assert_eq!(quad.border, Insets::all(3.0));
    } else {
        panic!("expected Paint::Quad");
    }
}

#[test]
fn test_set_button_state_event() {
    let (mut app, _) = app_with_theme(ThemeMode::Dark);
    let r = root(&mut app, 200.0, 100.0);

    let btn: Handle<Button> = app.spawn(r, button());
    app.tick();

    app.emit(SetButtonState(WidgetState::Focused), btn);
    app.tick();
    assert_eq!(app.widget::<Button>(btn).unwrap().state, WidgetState::Focused);
}

// ── 6. Raw builder visual properties ──────────────────────────────────────────

#[test]
fn test_button_builder_properties() {
    let (mut app, _) = app_with_theme(ThemeMode::Dark);
    let r = root(&mut app, 200.0, 100.0);

    let bg = Color::rgb(0.2, 0.3, 0.4);
    let border_c = Color::rgb(0.5, 0.6, 0.7);
    let btn: Handle<Button> = app.spawn(
        r,
        button()
            .background(bg)
            .border(2.0, border_c)
            .radius(12.0)
            .padding(Insets::all(px(16.0)))
            .width(px(120.0))
            .height(px(48.0)),
    );
    app.tick();

    let w = app.widget::<Button>(btn).unwrap();
    assert_eq!(w.background(), Some(bg));
    assert_eq!(w.border_color(), Some(border_c));
    assert_eq!(w.border_thickness(), 2.0);
    assert_eq!(w.border_radius(), 12.0);
    assert_eq!(w.padding(), Some(Insets::all(px(16.0))));

    // Inspect the inner Div layout and paint.
    let div_h = w.div_handle().unwrap();
    let layout = app.component::<LayoutStyle>(div_h).unwrap();
    assert_eq!(layout.width, px(120.0));
    assert_eq!(layout.height, px(48.0));
    assert_eq!(layout.padding, Insets::all(px(16.0)));

    let paint = app.component::<Paint>(div_h).unwrap();
    if let Paint::Quad(quad) = paint {
        assert_eq!(quad.color, bg);
        assert_eq!(quad.border_color, border_c);
        assert_eq!(quad.border, Insets::all(2.0));
    } else {
        panic!("expected Paint::Quad");
    }
}

// ── 7. Custom layout ──────────────────────────────────────────────────────────

#[test]
fn test_button_custom_layout() {
    let (mut app, _) = app_with_theme(ThemeMode::Dark);
    let r = root(&mut app, 200.0, 100.0);

    let btn: Handle<Button> = app.spawn(
        r,
        button().layout(LayoutStyle::default().column().size(px(150.0), px(50.0))),
    );
    app.tick();

    let w = app.widget::<Button>(btn).unwrap();
    let div_h = w.div_handle().unwrap();
    let layout = app.component::<LayoutStyle>(div_h).unwrap();
    assert_eq!(layout.width, px(150.0));
    assert_eq!(layout.height, px(50.0));
}

// ── 8. Interaction does not depend on child content ──────────────────────────

#[test]
fn test_interaction_independent_of_content() {
    use widgets::prelude::text as native_text;

    let (mut app, font) = app_with_theme(ThemeMode::Dark);
    let r = root(&mut app, 200.0, 100.0);

    let btn: Handle<Button> = app.spawn(
        r,
        button().child(native_text(font, "Label").size(14).color(Color::WHITE)),
    );
    app.tick();

    assert_eq!(app.widget::<Button>(btn).unwrap().state, WidgetState::Enabled);

    app.emit(Enter { contact: ContactId::Mouse, position: Point::new(0.0, 0.0) }, btn);
    app.tick();
    assert_eq!(app.widget::<Button>(btn).unwrap().state, WidgetState::Hovered);

    app.emit(Press { contact: ContactId::Mouse, position: Point::new(0.0, 0.0) }, btn);
    app.tick();
    assert_eq!(app.widget::<Button>(btn).unwrap().state, WidgetState::Pressed);

    app.emit(Release { contact: ContactId::Mouse, position: Point::new(0.0, 0.0) }, btn);
    app.tick();
    assert_eq!(app.widget::<Button>(btn).unwrap().state, WidgetState::Hovered);
}

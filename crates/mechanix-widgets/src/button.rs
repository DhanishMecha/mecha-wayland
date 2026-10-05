//! A raw, generic interactive button container widget.
//!
//! # Architecture
//!
//! `Button` is a content-agnostic interactive container that coordinates:
//! - An inner container [`Div`] applying background, border, corner radius, and
//!   layout style.
//! - Interactive state management ([`WidgetState`]: `Enabled`, `Hovered`,
//!   `Focused`, `Pressed`, `Disabled`).
//! - Pointer, touch, and click events.
//! - Arbitrary child content supplied by the caller via `.child(builder)` or
//!   `.child_fn(...)`.
//!
//! Higher-level buttons (e.g. `MechanixButton`, `IconButton`) wrap `Button`
//! to provide design-system-specific tokens, variants, and default content.

use app::{Build, Context, Handle, NodeId, Spawner, Widget};
use geometry::{Color, Insets};
use interactivity::{Clicked, Enter, Exit, Press, Release};
use layout::{LayoutStyle, StyleContext, Val, px};
use widgets::prelude::{Div, DivContext, div};

pub use crate::state::{StateLayer, WidgetState};

// ── Events & Types ───────────────────────────────────────────────────────────

/// Event to set the [`WidgetState`] of a button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetButtonState(pub WidgetState);
impl app::Event for SetButtonState {}

/// Event emitted by [`Button`] whenever its [`WidgetState`] changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonStateChanged(pub WidgetState);
impl app::Event for ButtonStateChanged {}

/// Callback handler type for button click events.
pub type OnClickHandler = Box<dyn FnMut(&mut Context<'_, Button>) + 'static>;

/// A boxed function that spawns arbitrary child content under the inner [`Div`].
pub type ChildFn = Box<dyn FnOnce(&mut Spawner<'_, Button>, Handle<Div>) -> NodeId>;

// ── Button Widget ────────────────────────────────────────────────────────────

/// A raw, content-agnostic interactive container.
#[derive(Debug, Clone)]
pub struct Button {
    pub state: WidgetState,
    pub background: Option<Color>,
    pub border_color: Option<Color>,
    pub border_thickness: f32,
    pub border_radius: f32,
    pub padding: Option<Insets<Val>>,
    pub div_handle: Option<Handle<Div>>,
}

impl Button {
    pub fn div_handle(&self) -> Option<Handle<Div>> {
        self.div_handle
    }

    pub fn state(&self) -> WidgetState {
        self.state
    }

    pub fn is_disabled(&self) -> bool {
        self.state.is_disabled()
    }

    pub fn background(&self) -> Option<Color> {
        self.background
    }

    pub fn border_color(&self) -> Option<Color> {
        self.border_color
    }

    pub fn border_thickness(&self) -> f32 {
        self.border_thickness
    }

    pub fn border_radius(&self) -> f32 {
        self.border_radius
    }

    pub fn padding(&self) -> Option<Insets<Val>> {
        self.padding
    }
}

// ── Constructor function ──────────────────────────────────────────────────────

/// Create a raw [`Button`] builder.
pub fn button() -> ButtonBuilder {
    ButtonBuilder::new()
}

// ── ButtonBuilder ─────────────────────────────────────────────────────────────

/// Builder for constructing a raw [`Button`] widget.
pub struct ButtonBuilder {
    state: WidgetState,
    background: Option<Color>,
    border_color: Option<Color>,
    border_thickness: f32,
    border_radius: f32,
    padding: Option<Insets<Val>>,
    layout: Option<LayoutStyle>,
    width: Option<Val>,
    height: Option<Val>,
    on_click: Option<OnClickHandler>,
    child_fn: Option<ChildFn>,
}

impl ButtonBuilder {
    pub fn new() -> Self {
        Self {
            state: WidgetState::Enabled,
            background: None,
            border_color: None,
            border_thickness: 0.0,
            border_radius: 0.0,
            padding: None,
            layout: None,
            width: None,
            height: None,
            on_click: None,
            child_fn: None,
        }
    }

    /// Construct the layout for the container [`Div`].
    pub fn build_layout(&self) -> LayoutStyle {
        let mut l = self.layout.clone().unwrap_or_else(|| {
            LayoutStyle::default().row().center().gap(px(8.0))
        });
        if let Some(pad) = self.padding {
            l = l.padding(pad);
        }
        if let Some(w) = self.width {
            l = l.width(w);
        }
        if let Some(h) = self.height {
            l = l.height(h);
        }
        l
    }

    // ── Builder Setters ──────────────────────────────────────────────────────

    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub fn background_color(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub fn border(mut self, thickness: f32, color: Color) -> Self {
        self.border_thickness = thickness;
        self.border_color = Some(color);
        self
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self
    }

    pub fn border_thickness(mut self, thickness: f32) -> Self {
        self.border_thickness = thickness;
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.border_radius = radius;
        self
    }

    pub fn border_radius(mut self, radius: f32) -> Self {
        self.border_radius = radius;
        self
    }

    pub fn padding(mut self, padding: Insets<Val>) -> Self {
        self.padding = Some(padding);
        self
    }

    pub fn width(mut self, width: impl Into<Val>) -> Self {
        self.width = Some(width.into());
        self
    }

    pub fn height(mut self, height: impl Into<Val>) -> Self {
        self.height = Some(height.into());
        self
    }

    pub fn layout(mut self, layout: LayoutStyle) -> Self {
        self.layout = Some(layout);
        self
    }

    pub fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.state = if disabled {
            WidgetState::Disabled
        } else {
            WidgetState::Enabled
        };
        self
    }

    pub fn on_click(mut self, handler: impl FnMut(&mut Context<'_, Button>) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    pub fn child<B: Build>(mut self, builder: B) -> Self {
        self.child_fn = Some(Box::new(move |s, container| {
            s.spawn(container, builder).id()
        }));
        self
    }

    pub fn child_fn(
        mut self,
        f: impl FnOnce(&mut Spawner<'_, Button>, Handle<Div>) -> NodeId + 'static,
    ) -> Self {
        self.child_fn = Some(Box::new(f));
        self
    }
}

impl Default for ButtonBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Build for ButtonBuilder {
    type Widget = Button;
}

// ── Interaction Transitions ──────────────────────────────────────────────────

fn bind_pointer_transitions(s: &mut Spawner<'_, Button>, me: Handle<Button>) {
    s.on::<Enter>(me, |ctx, _| {
        if ctx.me().state == WidgetState::Enabled {
            ctx.set_button_state(WidgetState::Hovered);
        }
    });
    s.on::<Exit>(me, |ctx, _| {
        if matches!(ctx.me().state, WidgetState::Hovered | WidgetState::Pressed) {
            ctx.set_button_state(WidgetState::Enabled);
        }
    });
    s.on::<Press>(me, |ctx, _| {
        if matches!(
            ctx.me().state,
            WidgetState::Enabled | WidgetState::Hovered | WidgetState::Focused
        ) {
            ctx.set_button_state(WidgetState::Pressed);
        }
    });
    s.on::<Release>(me, |ctx, _| {
        if ctx.me().state == WidgetState::Pressed {
            ctx.set_button_state(WidgetState::Hovered);
        }
    });
}

// ── Widget Implementation ────────────────────────────────────────────────────

impl Widget for Button {
    type Builder = ButtonBuilder;

    fn build(b: ButtonBuilder, me: Handle<Button>, s: &mut Spawner<'_, Button>) -> Button {
        let mut div_builder = div().style(b.build_layout());

        if let Some(bg) = b.background {
            div_builder = div_builder.background(bg);
        }
        if let Some(bc) = b.border_color {
            div_builder = div_builder.border(b.border_thickness, bc);
        }
        if b.border_radius > 0.0 {
            div_builder = div_builder.radius(b.border_radius);
        }

        let div_handle: Handle<Div> = s.spawn(me, div_builder);

        if let Some(child_fn) = b.child_fn {
            child_fn(s, div_handle);
        }

        bind_pointer_transitions(s, me);

        if let Some(mut cb) = b.on_click {
            s.on::<Clicked>(me, move |ctx, _| {
                if ctx.me().state != WidgetState::Disabled {
                    cb(ctx);
                }
            });
        }

        s.on::<SetButtonState>(me, |ctx, e| ctx.set_button_state(e.0));

        Button {
            state: b.state,
            background: b.background,
            border_color: b.border_color,
            border_thickness: b.border_thickness,
            border_radius: b.border_radius,
            padding: b.padding,
            div_handle: Some(div_handle),
        }
    }
}

// ── ButtonContextExt ─────────────────────────────────────────────────────────

/// Post-spawn setters for [`Button`], implemented on `Context<'_, Button>`.
pub trait ButtonContextExt {
    /// Enable or disable the button at runtime.
    fn set_disabled(&mut self, disabled: bool);

    /// Set the interaction state directly at runtime.
    fn set_button_state(&mut self, state: WidgetState);

    /// Set the container background color at runtime.
    fn set_background(&mut self, color: Color);

    /// Set the container border at runtime.
    fn set_border(&mut self, thickness: f32, color: Color);

    /// Set the container corner radius at runtime.
    fn set_radius(&mut self, radius: f32);

    /// Set the container padding at runtime.
    fn set_container_padding(&mut self, padding: Insets<Val>);
}

pub use ButtonContextExt as ButtonContext;

impl ButtonContextExt for Context<'_, Button> {
    fn set_disabled(&mut self, disabled: bool) {
        let new_state = if disabled {
            WidgetState::Disabled
        } else {
            WidgetState::Enabled
        };
        self.set_button_state(new_state);
    }

    fn set_button_state(&mut self, new_state: WidgetState) {
        if self.me().state == new_state {
            return;
        }
        self.me().state = new_state;
        let me = self.handle();
        self.emit(ButtonStateChanged(new_state), me);
    }

    fn set_background(&mut self, color: Color) {
        self.me().background = Some(color);
        if let Some(h) = self.me().div_handle {
            if let Some(mut c) = self.at(h) {
                c.set_background(color);
            }
        }
    }

    fn set_border(&mut self, thickness: f32, color: Color) {
        self.me().border_thickness = thickness;
        self.me().border_color = Some(color);
        if let Some(h) = self.me().div_handle {
            if let Some(mut c) = self.at(h) {
                c.set_border(thickness, color);
            }
        }
    }

    fn set_radius(&mut self, radius: f32) {
        self.me().border_radius = radius;
        if let Some(h) = self.me().div_handle {
            if let Some(mut c) = self.at(h) {
                c.set_radius(radius);
            }
        }
    }

    fn set_container_padding(&mut self, padding: Insets<Val>) {
        self.me().padding = Some(padding);
        if let Some(h) = self.me().div_handle {
            if let Some(mut c) = self.at(h) {
                c.set_padding(padding);
            }
        }
    }
}

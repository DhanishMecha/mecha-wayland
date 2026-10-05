//! A Material 3 / Mechanix design token button widget built on top of the
//! raw [`Button`] widget.
//!
//! # Architecture
//!
//! `MechanixButton` wraps [`Button`] to provide design system tokens,
//! variants ([`MechanixButtonVariant::Filled`] and
//! [`MechanixButtonVariant::Outlined`]), state layer overlays, and reactive
//! theme resolution.
//!
//! It remains content-agnostic: child content is supplied via `.child(...)` or
//! `.child_fn(...)`.

use app::{Build, Context, Handle, NodeId, Spawner, Widget};
use geometry::{Color, Insets};
use interactivity::Clicked;
use layout::{LayoutStyle, Val, px};
use theme::{ColorRole, ColorScheme, MechanixTheme, Shape, SpawnerThemeExt, ThemeReader};
use widgets::prelude::Div;

use crate::button::{Button, ButtonBuilder, ButtonContextExt, ButtonStateChanged, button};
use crate::state::{StateLayer, WidgetState};

// ── MechanixButtonSize ────────────────────────────────────────────────────────

/// Size preset for a [`MechanixButton`]: controls container minimum height and
/// horizontal padding. `icon_size` is informational for callers rendering icons.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MechanixButtonSize {
    /// Button height in dp.
    pub height: f32,
    /// Horizontal and vertical padding edges / insets.
    pub padding: Insets<Val>,
    /// Recommended icon size in dp (informational).
    pub icon_size: f32,
}

impl MechanixButtonSize {
    pub const fn new(height: f32, pad_x: f32, icon_size: f32) -> Self {
        Self {
            height,
            padding: Insets::symmetric(px(pad_x), px(0.0)),
            icon_size,
        }
    }

    /// Extra Small button (28 dp height, 12 dp horizontal padding, 20 dp icon size).
    pub const EXTRA_SMALL: Self = Self::new(28.0, 12.0, 20.0);
    /// Small button (32 dp height, 16 dp horizontal padding, 20 dp icon size).
    pub const SMALL: Self = Self::new(32.0, 16.0, 20.0);
    /// Medium button (44 dp height, 24 dp horizontal padding, 24 dp icon size).
    pub const MEDIUM: Self = Self::new(44.0, 24.0, 24.0);
    /// Large button (72 dp height, 48 dp horizontal padding, 32 dp icon size).
    pub const LARGE: Self = Self::new(72.0, 48.0, 32.0);
    /// Extra Large button (100 dp height, 64 dp horizontal padding, 40 dp icon size).
    pub const EXTRA_LARGE: Self = Self::new(100.0, 64.0, 40.0);
}

impl Default for MechanixButtonSize {
    fn default() -> Self {
        Self::MEDIUM
    }
}

pub use MechanixButtonSize as ButtonSize;

// ── MechanixButtonVariant ─────────────────────────────────────────────────────

/// Mechanix button variant presets (Filled and Outlined).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MechanixButtonVariant {
    #[default]
    Filled,
    Outlined,
}

pub use MechanixButtonVariant as ButtonVariant;

// ── MechanixButtonStyle ───────────────────────────────────────────────────────

/// Mechanix design token specification for a [`MechanixButton`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MechanixButtonStyle {
    /// Active variant preset.
    pub variant: MechanixButtonVariant,
    /// Container background color role when enabled (`None` for transparent container).
    pub background_color: Option<ColorRole>,
    /// Foreground / content color role when enabled (informational for child widgets).
    pub label_color: ColorRole,
    /// Border color role when enabled (`None` for no border).
    pub border_color: Option<ColorRole>,
    /// Border thickness in dp (`0.0` for Filled, `1.0`/`2.0` for Outlined).
    pub border_thickness: f32,

    /// Container background color role when disabled.
    pub disabled_background_color: Option<ColorRole>,
    /// Background opacity when disabled.
    pub disabled_background_opacity: f32,
    /// Label / icon color role when disabled.
    pub disabled_label_color: ColorRole,
    /// Label opacity when disabled.
    pub disabled_label_opacity: f32,
    /// Border color role when disabled.
    pub disabled_border_color: Option<ColorRole>,
    /// Border opacity when disabled.
    pub disabled_border_opacity: f32,

    /// State layer spec when hovered.
    pub hover_state_layer: StateLayer,
    /// State layer spec when focused.
    pub focus_state_layer: StateLayer,
    /// State layer spec when pressed.
    pub pressed_state_layer: StateLayer,

    /// Focus border color role (`None` for no focus border override).
    pub focus_border_color: Option<ColorRole>,
    /// Focus border thickness in dp.
    pub focus_border_thickness: f32,

    /// Shape token.
    pub shape: Shape,
    /// Size preset.
    pub size: MechanixButtonSize,
}

impl Default for MechanixButtonStyle {
    fn default() -> Self {
        Self::filled()
    }
}

impl MechanixButtonStyle {
    /// Switch variant while preserving current size and shape.
    pub fn with_variant(self, v: MechanixButtonVariant) -> Self {
        let base = match v {
            MechanixButtonVariant::Filled => Self::filled(),
            MechanixButtonVariant::Outlined => Self::outlined(),
        };
        Self {
            size: self.size,
            shape: self.shape,
            ..base
        }
    }

    /// Returns Filled Button style token set.
    pub fn filled() -> Self {
        let size = MechanixButtonSize::MEDIUM;
        Self {
            variant: MechanixButtonVariant::Filled,
            background_color: Some(ColorRole::SecondaryFixedDim),
            label_color: ColorRole::OnPrimary,
            border_color: None,
            border_thickness: 0.0,

            disabled_background_color: Some(ColorRole::OnSurface),
            disabled_background_opacity: 0.10,
            disabled_label_color: ColorRole::OnSurface,
            disabled_label_opacity: 0.38,
            disabled_border_color: None,
            disabled_border_opacity: 0.0,

            hover_state_layer: StateLayer {
                color_variant: ColorRole::OnPrimary,
                opacity: 0.08,
            },
            focus_state_layer: StateLayer {
                color_variant: ColorRole::OnPrimary,
                opacity: 0.10,
            },
            pressed_state_layer: StateLayer {
                color_variant: ColorRole::OnPrimary,
                opacity: 0.12,
            },

            focus_border_color: Some(ColorRole::Outline),
            focus_border_thickness: 3.0,

            shape: Shape::None,
            size,
        }
    }

    /// Returns Outlined Button style token set.
    pub fn outlined() -> Self {
        let size = MechanixButtonSize::MEDIUM;
        Self {
            variant: MechanixButtonVariant::Outlined,
            background_color: None,
            label_color: ColorRole::OnPrimary,
            border_color: Some(ColorRole::Outline),
            border_thickness: 2.0,

            disabled_background_color: None,
            disabled_background_opacity: 0.0,
            disabled_label_color: ColorRole::OnSurface,
            disabled_label_opacity: 0.38,
            disabled_border_color: Some(ColorRole::OnSurface),
            disabled_border_opacity: 0.10,

            hover_state_layer: StateLayer {
                color_variant: ColorRole::OnSurfaceVariant,
                opacity: 0.08,
            },
            focus_state_layer: StateLayer {
                color_variant: ColorRole::OnSurfaceVariant,
                opacity: 0.10,
            },
            pressed_state_layer: StateLayer {
                color_variant: ColorRole::OnSurfaceVariant,
                opacity: 0.08,
            },

            focus_border_color: Some(ColorRole::Primary),
            focus_border_thickness: 3.0,

            shape: Shape::None,
            size,
        }
    }

    /// Returns this button style with accented coloring applied for its variant.
    pub fn accented(mut self) -> Self {
        match self.variant {
            MechanixButtonVariant::Filled => {
                self.background_color = Some(ColorRole::Primary);
                self.label_color = ColorRole::OnPrimary;
                self.hover_state_layer.color_variant = ColorRole::OnPrimary;
                self.focus_state_layer.color_variant = ColorRole::OnPrimary;
                self.pressed_state_layer.color_variant = ColorRole::OnPrimary;
            }
            MechanixButtonVariant::Outlined => {
                self.border_color = Some(ColorRole::Primary);
                self.label_color = ColorRole::Primary;
                self.hover_state_layer.color_variant = ColorRole::Primary;
                self.focus_state_layer.color_variant = ColorRole::Primary;
                self.pressed_state_layer.color_variant = ColorRole::Primary;
            }
        }
        self
    }

    /// Resolve all style references and active overrides into a concrete [`ResolvedMechanixButtonStyle`].
    pub fn resolve(
        &self,
        theme: &MechanixTheme,
        state: WidgetState,
        overrides: &MechanixButtonOverrides,
    ) -> ResolvedMechanixButtonStyle {
        let scheme: &ColorScheme = &theme.colors;

        let base_background = overrides.background_color.unwrap_or_else(|| {
            self.background_color
                .map(|r| r.resolve(scheme))
                .unwrap_or(Color::TRANSPARENT)
        });

        let base_border = overrides.border_color.unwrap_or_else(|| {
            self.border_color
                .map(|r| r.resolve(scheme))
                .unwrap_or(Color::TRANSPARENT)
        });

        let border_thickness = overrides.border_thickness.unwrap_or(self.border_thickness);

        let border_radius = overrides.border_radius.unwrap_or_else(|| match self.shape {
            Shape::None => 0.0,
            Shape::Full => self.size.height / 2.0,
            other => other.resolve_radius_dp(self.size.height),
        });

        let base_content = overrides
            .content_color
            .unwrap_or_else(|| self.label_color.resolve(scheme));

        let mut style = ResolvedMechanixButtonStyle {
            background_color: base_background,
            border_color: base_border,
            border_thickness,
            border_radius,
            content_color: base_content,
        };

        match state {
            WidgetState::Enabled => {}
            WidgetState::Hovered => {
                style.background_color = self.hover_state_layer.apply(base_background, scheme);
            }
            WidgetState::Focused => {
                style.background_color = self.focus_state_layer.apply(base_background, scheme);
                if let Some(focus_border) = self.focus_border_color {
                    style.border_color = focus_border.resolve(scheme);
                    style.border_thickness = self.focus_border_thickness;
                }
            }
            WidgetState::Pressed => {
                style.background_color = self.pressed_state_layer.apply(base_background, scheme);
            }
            WidgetState::Disabled => {
                let fade = |c: Color, a: f32| Color::rgba(c.r, c.g, c.b, c.a * a);
                let pick = |ov: Option<Color>, role: Option<ColorRole>, a: f32| match (ov, role) {
                    (Some(c), _) => fade(c, a),
                    (None, Some(r)) => fade(r.resolve(scheme), a),
                    (None, None) => Color::TRANSPARENT,
                };
                style.background_color = pick(
                    overrides.background_color,
                    self.disabled_background_color,
                    self.disabled_background_opacity,
                );
                style.border_color = pick(
                    overrides.border_color,
                    self.disabled_border_color,
                    self.disabled_border_opacity,
                );
                style.content_color = pick(
                    overrides.content_color,
                    Some(self.disabled_label_color),
                    self.disabled_label_opacity,
                );
            }
        }

        style
    }
}

pub use MechanixButtonStyle as ButtonStyle;

// ── ResolvedMechanixButtonStyle ───────────────────────────────────────────────

/// A fully resolved, concrete Mechanix Button style ready for rendering.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedMechanixButtonStyle {
    /// Background fill color.
    pub background_color: Color,
    /// Border color.
    pub border_color: Color,
    /// Border thickness in pixels.
    pub border_thickness: f32,
    /// Resolved border radius in pixels.
    pub border_radius: f32,
    /// Content / text foreground color (for child widgets to consume).
    pub content_color: Color,
}

pub use ResolvedMechanixButtonStyle as ResolvedButtonStyle;

// ── MechanixButtonOverrides ───────────────────────────────────────────────────

/// Explicit style overrides provided by the user that take precedence over design tokens.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MechanixButtonOverrides {
    /// Container background color override.
    pub background_color: Option<Color>,
    /// Border color override.
    pub border_color: Option<Color>,
    /// Border thickness override in dp/px.
    pub border_thickness: Option<f32>,
    /// Border radius override in pixels.
    pub border_radius: Option<f32>,
    /// Content / label foreground color override.
    pub content_color: Option<Color>,
}

impl MechanixButtonOverrides {
    /// Clear explicit container, border, and content color overrides.
    pub fn clear_colors(&mut self) {
        self.background_color = None;
        self.border_color = None;
        self.content_color = None;
    }
}

pub use MechanixButtonOverrides as ButtonOverrides;

// ── MechanixButton Widget ─────────────────────────────────────────────────────

/// Callback handler type for mechanix button click events.
pub type OnClickHandler = Box<dyn FnMut(&mut Context<'_, MechanixButton>) + 'static>;

/// A boxed function that spawns arbitrary child content under the inner [`Div`].
pub type ChildFn = Box<dyn FnOnce(&mut Spawner<'_, Button>, Handle<Div>) -> NodeId>;

/// An M3 / Mechanix design token button widget wrapping [`Button`].
#[derive(Debug, Clone)]
pub struct MechanixButton {
    pub style: MechanixButtonStyle,
    pub overrides: MechanixButtonOverrides,
    pub state: WidgetState,
    pub button_handle: Option<Handle<Button>>,
}

impl MechanixButton {
    pub fn button_handle(&self) -> Option<Handle<Button>> {
        self.button_handle
    }

    pub fn state(&self) -> WidgetState {
        self.state
    }

    pub fn is_disabled(&self) -> bool {
        self.state.is_disabled()
    }

    pub fn variant(&self) -> MechanixButtonVariant {
        self.style.variant
    }

    pub fn size(&self) -> MechanixButtonSize {
        self.style.size
    }
}

// ── Constructor function ──────────────────────────────────────────────────────

/// Create a [`MechanixButton`] builder.
pub fn mechanix_button() -> MechanixButtonBuilder {
    MechanixButtonBuilder::new()
}

// ── MechanixButtonBuilder ─────────────────────────────────────────────────────

/// Builder for constructing a [`MechanixButton`] widget.
pub struct MechanixButtonBuilder {
    style: MechanixButtonStyle,
    overrides: MechanixButtonOverrides,
    state: WidgetState,
    layout: Option<LayoutStyle>,
    width: Option<Val>,
    height: Option<Val>,
    padding: Option<Insets<Val>>,
    on_click: Option<OnClickHandler>,
    child_fn: Option<ChildFn>,
}

impl MechanixButtonBuilder {
    pub fn new() -> Self {
        Self {
            style: MechanixButtonStyle::filled(),
            overrides: Default::default(),
            state: Default::default(),
            layout: None,
            width: None,
            height: None,
            padding: None,
            on_click: None,
            child_fn: None,
        }
    }

    pub fn variant(mut self, variant: MechanixButtonVariant) -> Self {
        self.style = self.style.with_variant(variant);
        self.overrides = MechanixButtonOverrides::default();
        self
    }

    pub fn size(mut self, size: MechanixButtonSize) -> Self {
        self.style.size = size;
        self
    }

    pub fn shape(mut self, shape: Shape) -> Self {
        self.style.shape = shape;
        self
    }

    pub fn accented(mut self) -> Self {
        self.style = self.style.accented();
        self.overrides.clear_colors();
        self
    }

    pub fn style(mut self, style: MechanixButtonStyle) -> Self {
        self.style = style;
        self.overrides = MechanixButtonOverrides::default();
        self
    }

    pub fn background_color(mut self, color: Color) -> Self {
        self.overrides.background_color = Some(color);
        self
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.overrides.border_color = Some(color);
        self
    }

    pub fn border_thickness(mut self, thickness: f32) -> Self {
        self.overrides.border_thickness = Some(thickness);
        self
    }

    pub fn border_radius(mut self, radius: f32) -> Self {
        self.overrides.border_radius = Some(radius);
        self
    }

    pub fn content_color(mut self, color: Color) -> Self {
        self.overrides.content_color = Some(color);
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

    pub fn on_click(
        mut self,
        handler: impl FnMut(&mut Context<'_, MechanixButton>) + 'static,
    ) -> Self {
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

impl Default for MechanixButtonBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Build for MechanixButtonBuilder {
    type Widget = MechanixButton;
}

// ── Widget Implementation ────────────────────────────────────────────────────

impl Widget for MechanixButton {
    type Builder = MechanixButtonBuilder;

    fn build(
        b: MechanixButtonBuilder,
        me: Handle<MechanixButton>,
        s: &mut Spawner<'_, MechanixButton>,
    ) -> MechanixButton {
        let initial_style = {
            let theme = s.theme();
            b.style.resolve(theme, b.state, &b.overrides)
        };

        let mut raw_btn: ButtonBuilder = button()
            .state(b.state)
            .background(initial_style.background_color)
            .border(initial_style.border_thickness, initial_style.border_color)
            .radius(initial_style.border_radius)
            .padding(b.padding.unwrap_or(b.style.size.padding));

        if let Some(w) = b.width {
            raw_btn = raw_btn.width(w);
        }
        if let Some(h) = b.height {
            raw_btn = raw_btn.height(h);
        } else {
            raw_btn = raw_btn.height(px(b.style.size.height));
        }

        if let Some(l) = b.layout {
            raw_btn = raw_btn.layout(l);
        }

        if let Some(child_fn) = b.child_fn {
            raw_btn = raw_btn.child_fn(child_fn);
        }

        let btn_handle: Handle<Button> = s.spawn(me, raw_btn);

        // When the underlying Button transitions interaction states, sync MechanixButton.
        s.on::<ButtonStateChanged>(btn_handle, move |ctx, e| {
            ctx.me().state = e.0;
            sync_mechanix_button(ctx);
        });

        // Theme switch listener.
        s.on_theme(me, sync_mechanix_button);

        // Forward on_click event.
        if let Some(mut cb) = b.on_click {
            s.on::<Clicked>(me, move |ctx, _| {
                if !ctx.me().is_disabled() {
                    cb(ctx);
                }
            });
        }

        MechanixButton {
            style: b.style,
            overrides: b.overrides,
            state: b.state,
            button_handle: Some(btn_handle),
        }
    }
}

// ── MechanixButtonContextExt ──────────────────────────────────────────────────

/// Post-spawn setters for [`MechanixButton`], implemented on `Context<'_, MechanixButton>`.
pub trait MechanixButtonContextExt {
    /// Enable or disable the button at runtime.
    fn set_disabled(&mut self, disabled: bool);

    /// Set the interaction state directly at runtime.
    fn set_button_state(&mut self, state: WidgetState);

    /// Change the button variant at runtime.
    fn set_variant(&mut self, variant: MechanixButtonVariant);

    /// Change the button size preset at runtime.
    fn set_size(&mut self, size: MechanixButtonSize);

    /// Resolve the current button style for the current state and overrides.
    fn resolved_style(&mut self) -> ResolvedMechanixButtonStyle;
}

pub use MechanixButtonContextExt as MechanixButtonContext;

impl MechanixButtonContextExt for Context<'_, MechanixButton> {
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
        sync_mechanix_button(self);
    }

    fn set_variant(&mut self, variant: MechanixButtonVariant) {
        let s = self.me().style.with_variant(variant);
        self.me().style = s;
        self.me().overrides = MechanixButtonOverrides::default();
        sync_mechanix_button(self);
    }

    fn set_size(&mut self, size: MechanixButtonSize) {
        self.me().style.size = size;
        sync_mechanix_button(self);
    }

    fn resolved_style(&mut self) -> ResolvedMechanixButtonStyle {
        let (style, state, overrides) = {
            let me = self.me();
            (me.style, me.state, me.overrides)
        };
        style.resolve(self.theme(), state, &overrides)
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn sync_mechanix_button(ctx: &mut Context<'_, MechanixButton>) {
    let (style, overrides, state, btn_h) = {
        let me = ctx.me();
        (me.style, me.overrides, me.state, me.button_handle)
    };
    let resolved = style.resolve(ctx.theme(), state, &overrides);

    if let Some(btn) = btn_h {
        if let Some(mut bctx) = ctx.at(btn) {
            bctx.set_background(resolved.background_color);
            bctx.set_border(resolved.border_thickness, resolved.border_color);
            bctx.set_radius(resolved.border_radius);
            bctx.set_container_padding(style.size.padding);
            bctx.set_button_state(state);
        }
    }
}

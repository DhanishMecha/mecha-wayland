//! A Material 3 / Mechanix design token button widget built with primitive [`div`],
//! [`text`], and [`icon`] widgets.
//!
//! # Architecture
//!
//! `Button` is a high-level, theme-aware widget that coordinates:
//! - A container [`Div`] applying layout, background, border, and corner radius tokens.
//! - An optional leading [`Icon`] tinted to match the content color and sized to token presets.
//! - A label [`Text`] resolved from theme typography tokens and font weight mappings.
//! - State layer compositing for interaction states ([`WidgetState`]: `Enabled`, `Hovered`, `Focused`, `Pressed`, `Disabled`).
//! - Automatic reactive re-styling on theme change events.

use crate::font::FontContextExt;
use app::{Build, Context, Handle, Spawner, Widget};
use atlas::{FontId, SpriteId};
use geometry::{Color, Insets, Size};
use interactivity::{Clicked, Enter, Exit, Press, Release};
use layout::{LayoutStyle, StyleContext, Val, px};
use theme::{
    ColorRole, ColorScheme, MechanixTheme, Shape, SpawnerThemeExt, TextVariant, ThemeReader,
};
use widgets::prelude::{
    Div, DivContext, Icon as NativeIcon, IconContext as NativeIconContext, Text as NativeText,
    TextContext as NativeTextContext, div, icon as native_icon, text as native_text,
};

// ── WidgetState ───────────────────────────────────────────────────────────────

/// The interactive state of a widget. Drives state-layer overlay and color
/// resolution for interactive M3 components across the widget library.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WidgetState {
    #[default]
    Enabled,
    Hovered,
    Focused,
    Pressed,
    Disabled,
}

impl WidgetState {
    /// Returns `true` when the widget should be drawn with a state-layer overlay.
    pub fn has_state_layer(self) -> bool {
        matches!(
            self,
            WidgetState::Hovered | WidgetState::Focused | WidgetState::Pressed
        )
    }

    /// Returns `true` when the widget is in the `Disabled` state.
    pub fn is_disabled(self) -> bool {
        matches!(self, WidgetState::Disabled)
    }
}

// ── StateLayer ────────────────────────────────────────────────────────────────

/// An M3 state-layer overlay: a semi-transparent tint applied on top of the
/// widget's background to communicate its current interaction state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StateLayer {
    /// The semantic color role whose resolved value is used as the overlay tint.
    pub color_variant: ColorRole,
    /// Opacity of the overlay in `[0, 1]`.
    pub opacity: f32,
}

impl StateLayer {
    /// Create a new `StateLayer` with the given color role and opacity.
    #[inline]
    pub const fn new(color_variant: ColorRole, opacity: f32) -> Self {
        Self {
            color_variant,
            opacity,
        }
    }

    /// Blend this state layer over `base_color` using the resolved tint from
    /// the given `scheme`. Uses source-over compositing.
    pub fn apply(self, base_color: Color, scheme: &ColorScheme) -> Color {
        let tint_rgb = self.color_variant.resolve(scheme);
        let overlay = Color::rgba(tint_rgb.r, tint_rgb.g, tint_rgb.b, self.opacity);
        overlay.over(base_color)
    }
}

// ── ButtonSize ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonSize {
    /// Button height in dp.
    pub height: f32,
    /// Horizontal and vertical padding edges / insets.
    pub padding: Insets<Val>,
    /// Recommended icon size in dp.
    pub icon_size: f32,
    /// Typography variant for the button text label.
    pub typography_variant: TextVariant,
}

impl ButtonSize {
    pub const fn new(
        height: f32,
        pad_x: f32,
        icon_size: f32,
        typography_variant: TextVariant,
    ) -> Self {
        Self {
            height,
            padding: Insets::symmetric(px(pad_x), px(0.0)),
            icon_size,
            typography_variant,
        }
    }

    /// Extra Small button (28 dp height, 12 dp horizontal padding, 20 dp icon size, `LabelLarge` typography).
    pub const EXTRA_SMALL: Self = Self::new(28.0, 12.0, 20.0, TextVariant::LabelLarge);
    /// Small button (32 dp height, 16 dp horizontal padding, 20 dp icon size, `LabelLarge` typography).
    pub const SMALL: Self = Self::new(32.0, 16.0, 20.0, TextVariant::LabelLarge);
    /// Medium button (44 dp height, 24 dp horizontal padding, 24 dp icon size, `TitleMedium` typography).
    pub const MEDIUM: Self = Self::new(44.0, 24.0, 24.0, TextVariant::TitleMedium);
    /// Large button (72 dp height, 48 dp horizontal padding, 32 dp icon size, `HeadlineSmall` typography).
    pub const LARGE: Self = Self::new(72.0, 48.0, 32.0, TextVariant::HeadlineSmall);
    /// Extra Large button (100 dp height, 64 dp horizontal padding, 40 dp icon size, `HeadlineLarge` typography).
    pub const EXTRA_LARGE: Self = Self::new(100.0, 64.0, 40.0, TextVariant::HeadlineLarge);
}

impl Default for ButtonSize {
    fn default() -> Self {
        Self::MEDIUM
    }
}

// ── ButtonVariant ─────────────────────────────────────────────────────────────

/// Button variant presets (Filled and Outlined).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    #[default]
    Filled,
    Outlined,
}

// ── ButtonStyle ───────────────────────────────────────────────────────────────

/// Mechanix design token specification for a [`Button`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonStyle {
    /// Active variant preset.
    pub variant: ButtonVariant,
    /// Container background color role when enabled (`None` for transparent container).
    pub background_color: Option<ColorRole>,
    /// Foreground label / icon color role when enabled.
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
    pub size: ButtonSize,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self::filled()
    }
}

impl ButtonStyle {
    /// Switch variant while preserving current size and shape.
    pub fn with_variant(self, v: ButtonVariant) -> Self {
        let base = match v {
            ButtonVariant::Filled => Self::filled(),
            ButtonVariant::Outlined => Self::outlined(),
        };
        Self {
            size: self.size,
            shape: self.shape,
            ..base
        }
    }

    /// Returns Filled Button style token set.
    pub fn filled() -> Self {
        let size = ButtonSize::MEDIUM;
        Self {
            variant: ButtonVariant::Filled,
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
        let size = ButtonSize::MEDIUM;
        Self {
            variant: ButtonVariant::Outlined,
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
            ButtonVariant::Filled => {
                self.background_color = Some(ColorRole::Primary);
                self.label_color = ColorRole::OnPrimary;
                self.hover_state_layer.color_variant = ColorRole::OnPrimary;
                self.focus_state_layer.color_variant = ColorRole::OnPrimary;
                self.pressed_state_layer.color_variant = ColorRole::OnPrimary;
            }
            ButtonVariant::Outlined => {
                self.border_color = Some(ColorRole::Primary);
                self.label_color = ColorRole::Primary;
                self.hover_state_layer.color_variant = ColorRole::Primary;
                self.focus_state_layer.color_variant = ColorRole::Primary;
                self.pressed_state_layer.color_variant = ColorRole::Primary;
            }
        }
        self
    }

    /// Resolve all style references and active overrides into a concrete [`ResolvedButtonStyle`].
    pub fn resolve(
        &self,
        theme: &MechanixTheme,
        state: WidgetState,
        overrides: &ButtonOverrides,
    ) -> ResolvedButtonStyle {
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

        let mut style = ResolvedButtonStyle {
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

// ── ResolvedButtonStyle ───────────────────────────────────────────────────────

/// A fully resolved, concrete Button style ready for rendering.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedButtonStyle {
    /// Background fill color.
    pub background_color: Color,
    /// Border color.
    pub border_color: Color,
    /// Border thickness in pixels.
    pub border_thickness: f32,
    /// Resolved border radius in pixels.
    pub border_radius: f32,
    /// Content / text foreground color.
    pub content_color: Color,
}

// ── ButtonOverrides ───────────────────────────────────────────────────────────

/// Explicit style overrides provided by the user that take precedence over design tokens.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ButtonOverrides {
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

impl ButtonOverrides {
    /// Clear explicit container, border, and content color overrides.
    pub fn clear_colors(&mut self) {
        self.background_color = None;
        self.border_color = None;
        self.content_color = None;
    }
}

// ── Events & Types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetButtonState(pub WidgetState);
impl app::Event for SetButtonState {}

/// Callback handler type for button click events.
pub type OnClickHandler = Box<dyn FnMut(&mut Context<'_, Button>) + 'static>;

// ── Button Widget ────────────────────────────────────────────────────────────

/// A Mechanix button widget holding design tokens and interaction state,
/// composed of primitive [`Div`], [`NativeText`], and optional [`NativeIcon`].
#[derive(Debug, Clone)]
pub struct Button {
    pub tokens: ButtonStyle,
    pub overrides: ButtonOverrides,
    pub state: WidgetState,
    pub font: Option<FontId>,
    pub label: String,
    pub icon: Option<SpriteId>,
    pub div_handle: Option<Handle<Div>>,
    pub label_handle: Option<Handle<NativeText>>,
    pub icon_handle: Option<Handle<NativeIcon>>,
}

impl Button {
    pub fn div_handle(&self) -> Option<Handle<Div>> {
        self.div_handle
    }

    pub fn label_handle(&self) -> Option<Handle<NativeText>> {
        self.label_handle
    }

    pub fn icon_handle(&self) -> Option<Handle<NativeIcon>> {
        self.icon_handle
    }

    pub fn state(&self) -> WidgetState {
        self.state
    }

    pub fn is_disabled(&self) -> bool {
        self.state.is_disabled()
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn variant(&self) -> ButtonVariant {
        self.tokens.variant
    }

    pub fn size(&self) -> ButtonSize {
        self.tokens.size
    }
}

/// Create a [`Button`] builder with the given text label (defaults to filled).
///
/// Typography and font are automatically resolved from the theme and [`FontBook`].
/// Explicit font overrides can be specified using `.font(...)`.
pub fn button(label: impl Into<String>) -> ButtonBuilder {
    ButtonBuilder::new(label)
}

/// Builder for constructing a [`Button`] widget.
pub struct ButtonBuilder {
    font: Option<FontId>,
    tokens: ButtonStyle,
    overrides: ButtonOverrides,
    state: WidgetState,
    layout: Option<LayoutStyle>,
    width: Option<Val>,
    height: Option<Val>,
    label: String,
    icon: Option<SpriteId>,
    on_click: Option<OnClickHandler>,
}

impl ButtonBuilder {
    /// Create a new filled button builder.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            font: None,
            tokens: ButtonStyle::filled(),
            overrides: Default::default(),
            state: Default::default(),
            layout: None,
            width: None,
            height: None,
            label: label.into(),
            icon: None,
            on_click: None,
        }
    }

    /// Construct the layout for the container [`Div`].
    pub fn build_layout(&self) -> LayoutStyle {
        let mut l = self.layout.clone().unwrap_or_else(|| {
            LayoutStyle::default()
                .row()
                .center()
                .min_height(px(self.tokens.size.height))
                .padding(self.tokens.size.padding)
                .gap(px(8.0))
        });
        if let Some(w) = self.width {
            l = l.width(w);
        }
        if let Some(h) = self.height {
            l = l.height(h);
        }
        l
    }

    // ── Builder Setters ──────────────────────────────────────────────────────

    /// Set container width.
    pub fn width(mut self, width: impl Into<Val>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Set container height.
    pub fn height(mut self, height: impl Into<Val>) -> Self {
        self.height = Some(height.into());
        self
    }

    /// Set button size preset ([`ButtonSize::SMALL`], [`ButtonSize::MEDIUM`], [`ButtonSize::LARGE`], etc.).
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.tokens.size = size;
        self
    }

    /// Set explicit container padding insets.
    pub fn padding(mut self, padding: Insets<Val>) -> Self {
        self.tokens.size.padding = padding;
        self
    }

    /// Set explicit background color.
    pub fn background_color(mut self, color: Color) -> Self {
        self.overrides.background_color = Some(color);
        self
    }

    /// Set explicit border color.
    pub fn border_color(mut self, color: Color) -> Self {
        self.overrides.border_color = Some(color);
        self
    }

    /// Set explicit border thickness in dp/px.
    pub fn border_thickness(mut self, thickness: f32) -> Self {
        self.overrides.border_thickness = Some(thickness);
        self
    }

    /// Set explicit border radius in pixels.
    pub fn border_radius(mut self, radius: f32) -> Self {
        self.overrides.border_radius = Some(radius);
        self
    }

    /// Set explicit content / label / icon color.
    pub fn content_color(mut self, color: Color) -> Self {
        self.overrides.content_color = Some(color);
        self
    }

    /// Set whole layout style, resetting explicit `width` and `height` overrides.
    pub fn layout(mut self, layout: LayoutStyle) -> Self {
        self.layout = Some(layout);
        self.width = None;
        self.height = None;
        self
    }

    /// Set variant preset ([`ButtonVariant::Filled`] or [`ButtonVariant::Outlined`]).
    /// Resets explicit color overrides.
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.tokens = self.tokens.with_variant(variant);
        self.overrides = ButtonOverrides::default();
        self
    }

    /// Set button shape token.
    pub fn shape(mut self, shape: Shape) -> Self {
        self.tokens.shape = shape;
        self
    }

    /// Replace the entire design tokens specification with a custom [`ButtonStyle`].
    pub fn style(mut self, tokens: ButtonStyle) -> Self {
        self.tokens = tokens;
        self.overrides = ButtonOverrides::default();
        self
    }

    /// Enable accented styling mode.
    pub fn accented(mut self) -> Self {
        self.tokens = self.tokens.accented();
        self.overrides.clear_colors();
        self
    }

    /// Set focus border color role.
    pub fn focus_border_color(mut self, color: ColorRole) -> Self {
        self.tokens.focus_border_color = Some(color);
        self
    }

    /// Set focus border thickness in dp.
    pub fn focus_border_thickness(mut self, thickness: f32) -> Self {
        self.tokens.focus_border_thickness = thickness;
        self
    }

    /// Override the font face used for the button label text.
    pub fn font(mut self, font: FontId) -> Self {
        self.font = Some(font);
        self
    }

    /// Set a leading icon on the button.
    pub fn icon(mut self, icon: SpriteId) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Set interaction state ([`WidgetState`]).
    pub fn state(mut self, state: WidgetState) -> Self {
        self.state = state;
        self
    }

    /// Disable the button initially.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.state = if disabled {
            WidgetState::Disabled
        } else {
            WidgetState::Enabled
        };
        self
    }

    /// Attach an `on_click` callback.
    pub fn on_click(mut self, handler: impl FnMut(&mut Context<'_, Button>) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
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
        let (button_style, font_size, font_weight) = {
            let theme = s.theme();
            let typo = b.tokens.size.typography_variant.resolve(&theme.typography);
            (
                b.tokens.resolve(theme, b.state, &b.overrides),
                typo.font_size.round() as u16,
                typo.weight,
            )
        };

        let font = b.font.unwrap_or_else(|| s.font(font_weight));

        let div_handle: Handle<Div> = s.spawn(
            me,
            div()
                .style(b.build_layout())
                .background(button_style.background_color)
                .border(button_style.border_thickness, button_style.border_color)
                .radius(button_style.border_radius),
        );

        let icon_handle = b.icon.map(|sprite| {
            let icon_sz = b.tokens.size.icon_size;
            s.spawn(
                div_handle,
                native_icon(sprite)
                    .color(button_style.content_color)
                    .style(LayoutStyle::default().size(px(icon_sz), px(icon_sz))),
            )
        });

        let label_handle = Some(
            s.spawn(
                div_handle,
                native_text(font, &b.label)
                    .size(font_size)
                    .color(button_style.content_color),
            ),
        );

        bind_pointer_transitions(s, me);

        if let Some(mut cb) = b.on_click {
            s.on::<Clicked>(me, move |ctx, _| {
                if ctx.me().state != WidgetState::Disabled {
                    cb(ctx);
                }
            });
        }

        s.on::<SetButtonState>(me, |ctx, e| ctx.set_button_state(e.0));
        s.on_theme(me, sync_theme);

        Button {
            tokens: b.tokens,
            overrides: b.overrides,
            state: b.state,
            font: b.font,
            label: b.label,
            icon: b.icon,
            div_handle: Some(div_handle),
            label_handle,
            icon_handle,
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

    /// Change the button variant at runtime.
    fn set_variant(&mut self, variant: ButtonVariant);

    /// Change the button size preset at runtime.
    fn set_size(&mut self, size: ButtonSize);

    /// Set or update the label text at runtime.
    fn set_label(&mut self, text: impl Into<String>);

    /// Set or update the leading icon at runtime.
    fn set_icon(&mut self, icon: SpriteId);
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
        sync_button(self);
    }

    fn set_variant(&mut self, variant: ButtonVariant) {
        let t = self.me().tokens.with_variant(variant);
        self.me().tokens = t;
        self.me().overrides = ButtonOverrides::default();
        sync_theme(self);
    }

    fn set_size(&mut self, size: ButtonSize) {
        self.me().tokens.size = size;
        sync_theme(self);
    }

    fn set_label(&mut self, text: impl Into<String>) {
        let s = text.into();
        self.me().label = s.clone();
        if let Some(h) = self.me().label_handle {
            if let Some(mut c) = self.at(h) {
                c.set_text(s);
            }
        }
    }

    fn set_icon(&mut self, icon: SpriteId) {
        self.me().icon = Some(icon);
        if let Some(h) = self.me().icon_handle {
            if let Some(mut c) = self.at(h) {
                c.set_sprite(icon);
            }
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn sync_button(ctx: &mut Context<'_, Button>) {
    let (tokens, state, overrides) = {
        let me = ctx.me();
        (me.tokens, me.state, me.overrides)
    };
    let style_spec = tokens.resolve(ctx.theme(), state, &overrides);

    if let Some(h) = ctx.me().div_handle {
        if let Some(mut c) = ctx.at(h) {
            c.set_background(style_spec.background_color);
            c.set_border(style_spec.border_thickness, style_spec.border_color);
            c.set_radius(style_spec.border_radius);
        }
    }

    let fg = style_spec.content_color;
    if let Some(h) = ctx.me().label_handle {
        if let Some(mut c) = ctx.at(h) {
            c.set_color(fg);
        }
    }

    if let Some(h) = ctx.me().icon_handle {
        if let Some(mut c) = ctx.at(h) {
            c.set_color(fg);
        }
    }
}

fn sync_theme(ctx: &mut Context<'_, Button>) {
    sync_button(ctx);
    let (tokens, font_override) = {
        let me = ctx.me();
        (me.tokens, me.font)
    };
    let typo = tokens
        .size
        .typography_variant
        .resolve(&ctx.theme().typography);
    let font_size = typo.font_size.round() as u16;
    let font = font_override.unwrap_or_else(|| ctx.font(typo.weight));

    if let Some(h) = ctx.me().label_handle {
        if let Some(mut c) = ctx.at(h) {
            c.set_font(font);
            c.set_size(font_size);
        }
    }

    if let Some(h) = ctx.me().div_handle {
        if let Some(mut c) = ctx.at(h) {
            c.set_min_height(px(tokens.size.height));
            c.set_padding(tokens.size.padding);
        }
    }

    if let Some(h) = ctx.me().icon_handle {
        if let Some(mut c) = ctx.at(h) {
            c.set_size(Size::new(tokens.size.icon_size, tokens.size.icon_size));
        }
    }
}

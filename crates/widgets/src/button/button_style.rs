use crate::state::StateLayer;
pub use crate::state::WidgetState;
use geometry::{Color, Insets};
use layout::{Val, px};
use theme::{ColorRole, ColorScheme, MechanixTheme, Shape, TextVariant};

// ButtonSize

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

// ButtonVariant

/// Button variant presets (Filled and Outlined).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    #[default]
    Filled,
    Outlined,
}

// ButtonStyle

/// Mechanix design token specification for a [`Button`](super::Button).
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
            disabled_label_opacity: 0.10,
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

            disabled_background_color: Some(ColorRole::OnSurface),
            disabled_background_opacity: 0.10,
            disabled_label_color: ColorRole::OnSurface,
            disabled_label_opacity: 1.0,
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

            focus_border_color: Some(ColorRole::Outline),
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

// ResolvedButtonStyle

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

// ButtonOverrides

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

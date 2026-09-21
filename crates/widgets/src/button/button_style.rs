use layout::{Val, px};
use theme::{ColorScheme, MechanixTheme, Shape, TextVariant, ThemeColor};
use utils::{Color, Edges};

use crate::utils::{StateLayer, WidgetState};

/// Predefined Material 3 height, padding, icon size, and typography presets for buttons.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonSize {
    /// Button height in dp.
    pub height: f32,
    /// Horizontal and vertical padding edges.
    pub padding: Edges<Val>,
    /// Recommended icon size in dp.
    pub icon_size: f32,
    /// Typography variant for the button text label.
    pub typography_variant: TextVariant,
}

impl ButtonSize {
    /// Extra Small button (28 dp height, 12 dp horizontal padding, 20 dp icon size, `LabelLarge` typography).
    pub const EXTRA_SMALL: Self = Self {
        height: 28.0,
        padding: Edges::symmetric(px(12.0), px(0.0)),
        icon_size: 20.0,
        typography_variant: TextVariant::LabelLarge,
    };

    /// Small button (32 dp height, 16 dp horizontal padding, 20 dp icon size, `LabelLarge` typography).
    pub const SMALL: Self = Self {
        height: 32.0,
        padding: Edges::symmetric(px(16.0), px(0.0)),
        icon_size: 20.0,
        typography_variant: TextVariant::LabelLarge,
    };

    /// Medium button (44 dp height, 24 dp horizontal padding, 24 dp icon size, `TitleMedium` typography).
    pub const MEDIUM: Self = Self {
        height: 44.0,
        padding: Edges::symmetric(px(24.0), px(0.0)),
        icon_size: 24.0,
        typography_variant: TextVariant::TitleMedium,
    };

    /// Large button (72 dp height, 48 dp horizontal padding, 32 dp icon size, `HeadlineSmall` typography).
    pub const LARGE: Self = Self {
        height: 72.0,
        padding: Edges::symmetric(px(48.0), px(0.0)),
        icon_size: 32.0,
        typography_variant: TextVariant::HeadlineSmall,
    };

    /// Extra Large button (100 dp height, 64 dp horizontal padding, 40 dp icon size, `HeadlineLarge` typography).
    pub const EXTRA_LARGE: Self = Self {
        height: 100.0,
        padding: Edges::symmetric(px(64.0), px(0.0)),
        icon_size: 40.0,
        typography_variant: TextVariant::HeadlineLarge,
    };
}

impl Default for ButtonSize {
    fn default() -> Self {
        Self::MEDIUM
    }
}

// ── ButtonStyle ──────────────────────────────────────────────────────────────

/// Material 3 design token specification for a [`Button`](super::Button).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonStyle {
    /// Container background color role when enabled (`None` for transparent container).
    pub background_color: Option<ThemeColor>,
    /// Foreground label / icon color role when enabled.
    pub label_color: ThemeColor,
    /// Border color role when enabled (`None` for no border).
    pub border_color: Option<ThemeColor>,
    /// Border thickness in dp (`0.0` for Filled, `2.0` for Outlined).
    pub border_thickness: f32,

    /// Container background color role when disabled.
    pub disabled_background_color: Option<ThemeColor>,
    /// Background opacity when disabled.
    pub disabled_background_opacity: f32,
    /// Label / icon color role when disabled.
    pub disabled_label_color: ThemeColor,
    /// Label opacity when disabled.
    pub disabled_label_opacity: f32,
    /// Border color role when disabled.
    pub disabled_border_color: Option<ThemeColor>,
    /// Border opacity when disabled.
    pub disabled_border_opacity: f32,

    /// State layer spec when hovered.
    pub hover_state_layer: StateLayer,
    /// State layer spec when focused.
    pub focus_state_layer: StateLayer,
    /// State layer spec when pressed.
    pub pressed_state_layer: StateLayer,

    /// Focus border color role (`None` for no focus border override).
    pub focus_border_color: Option<ThemeColor>,
    /// Focus border thickness in dp.
    pub focus_border_thickness: f32,

    /// Shape token.
    pub shape: Shape,
    /// Size preset.
    pub size: ButtonSize,
}

impl ButtonStyle {
    /// Returns **Filled Button** style token set.
    pub fn filled() -> Self {
        let size = ButtonSize::MEDIUM;
        Self {
            background_color: Some(ThemeColor::SecondaryFixedDim),
            label_color: ThemeColor::OnPrimary,
            border_color: None,
            border_thickness: 0.0,

            disabled_background_color: Some(ThemeColor::OnSurface),
            disabled_background_opacity: 0.10,
            disabled_label_color: ThemeColor::OnSurface,
            disabled_label_opacity: 0.10,
            disabled_border_color: None,
            disabled_border_opacity: 0.0,

            hover_state_layer: StateLayer {
                color_variant: ThemeColor::OnPrimary,
                opacity: 0.08,
            },
            focus_state_layer: StateLayer {
                color_variant: ThemeColor::OnPrimary,
                opacity: 0.10,
            },
            pressed_state_layer: StateLayer {
                color_variant: ThemeColor::OnPrimary,
                opacity: 0.12,
            },

            focus_border_color: Some(ThemeColor::Outline),
            focus_border_thickness: 3.0,

            shape: Shape::None,
            size,
        }
    }

    /// Returns **Outlined Button** style token set.
    pub fn outlined() -> Self {
        let size = ButtonSize::MEDIUM;
        Self {
            background_color: None,
            label_color: ThemeColor::OnPrimary,
            border_color: Some(ThemeColor::Outline),
            border_thickness: 2.0,

            disabled_background_color: Some(ThemeColor::OnSurface),
            disabled_background_opacity: 0.10,
            disabled_label_color: ThemeColor::OnSurface,
            disabled_label_opacity: 1.0,
            disabled_border_color: Some(ThemeColor::OnSurface),
            disabled_border_opacity: 0.10,

            hover_state_layer: StateLayer {
                color_variant: ThemeColor::OnSurfaceVariant,
                opacity: 0.08,
            },
            focus_state_layer: StateLayer {
                color_variant: ThemeColor::OnSurfaceVariant,
                opacity: 0.10,
            },
            pressed_state_layer: StateLayer {
                color_variant: ThemeColor::OnSurfaceVariant,
                opacity: 0.08,
            },

            focus_border_color: Some(ThemeColor::Secondary),
            focus_border_thickness: 3.0,

            shape: Shape::None,
            size,
        }
    }

    /// Resolve all style references into a concrete [`ResolvedButtonStyle`].
    pub fn resolve(&self, theme: &MechanixTheme, state: WidgetState) -> ResolvedButtonStyle {
        let scheme: &ColorScheme = &theme.colors;

        let base_background = self
            .background_color
            .map(|r| r.resolve(scheme))
            .unwrap_or(Color::TRANSPARENT);
        let base_border = self
            .border_color
            .map(|r| r.resolve(scheme))
            .unwrap_or(Color::TRANSPARENT);

        let mut style = ResolvedButtonStyle {
            background_color: base_background,
            border_color: base_border,
            border_thickness: self.border_thickness,
            border_radius: None,
            content_color: self.label_color.resolve(scheme),
            padding: self.size.padding,
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
                style.background_color = self
                    .disabled_background_color
                    .map(|r| {
                        let c = r.resolve(scheme);
                        Color::rgba(c.r, c.g, c.b, self.disabled_background_opacity)
                    })
                    .unwrap_or(Color::TRANSPARENT);

                style.border_color = self
                    .disabled_border_color
                    .map(|r| {
                        let c = r.resolve(scheme);
                        Color::rgba(c.r, c.g, c.b, self.disabled_border_opacity)
                    })
                    .unwrap_or(Color::TRANSPARENT);

                let base_label = self.disabled_label_color.resolve(scheme);
                style.content_color = Color::rgba(
                    base_label.r,
                    base_label.g,
                    base_label.b,
                    self.disabled_label_opacity,
                );
            }
        }

        style
    }
}

// ── ResolvedButtonStyle ──────────────────────────────────────────────────────

/// A fully resolved, concrete Button style ready for rendering.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedButtonStyle {
    /// Background fill color.
    pub background_color: Color,
    /// Border color.
    pub border_color: Color,
    /// Border thickness in pixels.
    pub border_thickness: f32,
    /// Optional explicit border radius.
    pub border_radius: Option<f32>,
    /// Content / text foreground color.
    pub content_color: Color,
    /// Padding edges.
    pub padding: Edges<Val>,
}

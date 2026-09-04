use taffy::{LengthPercentage, Rect as TaffyRect};
use theme::{
    ColorScheme, ColorVariant, EdgeInsets, MechanixTheme, Shape, StateLayer, TextVariant,
    WidgetState,
};
use utils::Color;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonSize {
    pub height: f32,
    pub padding: TaffyRect<LengthPercentage>,
    pub icon_size: f32,
    pub typography_variant: TextVariant,
}

impl ButtonSize {
    /// Extra Small button (28 dp height, 12 dp horizontal padding, 20 dp icon size).
    pub const EXTRA_SMALL: Self = Self {
        height: 28.0,
        padding: EdgeInsets::symmetric(12.0, 0.0),
        icon_size: 20.0,
        typography_variant: TextVariant::LabelLarge,
    };

    /// Small button (32 dp height, 16 dp horizontal padding, 20 dp icon size).
    pub const SMALL: Self = Self {
        height: 32.0,
        padding: EdgeInsets::symmetric(16.0, 0.0),
        icon_size: 20.0,
        typography_variant: TextVariant::LabelLarge,
    };

    /// Medium button (44 dp height, 24 dp horizontal padding, 24 dp icon size).
    pub const MEDIUM: Self = Self {
        height: 44.0,
        padding: EdgeInsets::symmetric(24.0, 0.0),
        icon_size: 24.0,
        typography_variant: TextVariant::TitleMedium,
    };

    /// Large button (72 dp height, 48 dp horizontal padding, 32 dp icon size).
    pub const LARGE: Self = Self {
        height: 72.0,
        padding: EdgeInsets::symmetric(48.0, 0.0),
        icon_size: 32.0,
        typography_variant: TextVariant::HeadlineSmall,
    };

    /// Extra Large button (100 dp height, 64 dp horizontal padding, 40 dp icon size).
    pub const EXTRA_LARGE: Self = Self {
        height: 100.0,
        padding: EdgeInsets::symmetric(64.0, 0.0),
        icon_size: 40.0,
        typography_variant: TextVariant::HeadlineLarge,
    };
}

impl Default for ButtonSize {
    fn default() -> Self {
        Self::MEDIUM
    }
}

//  ButtonStyle

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonStyle {
    // Enabled colors
    /// Container background color when enabled (`None` for transparent container).
    pub background_color: Option<ColorVariant>,
    /// Label / icon foreground color when enabled.
    pub label_color: ColorVariant,
    /// Border / outline color when enabled (`None` for no border).
    pub border_color: Option<ColorVariant>,
    /// Border thickness in dp (e.g. `1.0` for Outlined, `0.0` for Filled).
    pub border_thickness: f32,

    // Disabled
    pub disabled_background_color: Option<ColorVariant>,
    pub disabled_background_opacity: f32,
    pub disabled_label_color: ColorVariant,
    pub disabled_label_opacity: f32,
    pub disabled_border_color: Option<ColorVariant>,
    pub disabled_border_opacity: f32,

    pub hover_state_layer: StateLayer,
    pub focus_state_layer: StateLayer,
    pub pressed_state_layer: StateLayer,

    /// Focus border color (`None` for no border on focus).
    pub focus_border_color: Option<ColorVariant>,
    /// Focus border thickness in dp.
    pub focus_border_thickness: f32,

    pub shape: Shape,
    pub size: ButtonSize,
}

impl ButtonStyle {
    /// Returns **Filled Button** style set.
    pub fn filled() -> Self {
        let size = ButtonSize::MEDIUM;
        Self {
            background_color: Some(ColorVariant::SecondaryFixedDim),
            label_color: ColorVariant::OnPrimary,
            border_color: None,
            border_thickness: 0.0,

            disabled_background_color: Some(ColorVariant::OnSurface),
            disabled_background_opacity: 0.10,
            disabled_label_color: ColorVariant::OnSurface,
            disabled_label_opacity: 0.10,
            disabled_border_color: None,
            disabled_border_opacity: 0.0,

            hover_state_layer: StateLayer {
                color_variant: ColorVariant::OnPrimary,
                opacity: 0.08,
            },
            focus_state_layer: StateLayer {
                color_variant: ColorVariant::OnPrimary,
                opacity: 0.10,
            },
            pressed_state_layer: StateLayer {
                color_variant: ColorVariant::OnPrimary,
                opacity: 0.12,
            },

            focus_border_color: Some(ColorVariant::Outline),
            focus_border_thickness: 3.0,

            shape: Shape::None,
            size,
        }
    }

    /// Returns **Outlined Button** style set.
    pub fn outlined() -> Self {
        let size = ButtonSize::MEDIUM;
        Self {
            background_color: None,
            label_color: ColorVariant::OnPrimary,
            border_color: Some(ColorVariant::Outline),
            border_thickness: 2.0,

            disabled_background_color: Some(ColorVariant::OnSurface),
            disabled_background_opacity: 0.10,
            disabled_label_color: ColorVariant::OnSurface,
            disabled_label_opacity: 1.0,
            disabled_border_color: Some(ColorVariant::OnSurface),
            disabled_border_opacity: 0.10,

            hover_state_layer: StateLayer {
                color_variant: ColorVariant::OnSurfaceVariant,
                opacity: 0.08,
            },
            focus_state_layer: StateLayer {
                color_variant: ColorVariant::OnSurfaceVariant,
                opacity: 0.10,
            },
            pressed_state_layer: StateLayer {
                color_variant: ColorVariant::OnSurfaceVariant,
                opacity: 0.08,
            },

            focus_border_color: Some(ColorVariant::Secondary),
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

//  ResolvedButtonStyle

/// A fully resolved, renderer-ready Button style.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedButtonStyle {
    /// Background fill of the button container (with state layer blended in).
    pub background_color: Color,
    /// Border / outline color.
    pub border_color: Color,
    /// Border thickness in pixels.
    pub border_thickness: f32,
    /// Optional explicit border radius.
    pub border_radius: Option<f32>,
    /// Foreground color for label text and icons.
    pub content_color: Color,
    /// Container padding.
    pub padding: TaffyRect<LengthPercentage>,
}

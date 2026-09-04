use theme::{ColorVariant, FontWeight, MechanixTheme, TextVariant, TypographyStyle};
use utils::Color;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextStyle {
    pub color_variant: ColorVariant,
    pub color_override: Option<Color>,
    pub typography_variant: TextVariant,
    pub weight: Option<FontWeight>,
    pub is_emphasised: bool,
    pub line_height: Option<f32>,
    pub letter_spacing: Option<f32>,
    pub word_spacing: Option<f32>,
}

impl Default for TextStyle {
    /// M3 default: `OnSurface` color, `BodyMedium` typography.
    fn default() -> Self {
        Self {
            color_variant: ColorVariant::OnSurface,
            color_override: None,
            typography_variant: TextVariant::BodyMedium,
            weight: None,
            is_emphasised: false,
            line_height: None,
            letter_spacing: None,
            word_spacing: None,
        }
    }
}

impl TextStyle {
    /// Resolve style references into a concrete [`ResolvedTextStyle`].
    ///
    /// No `Color` literals are used here — all values come from the theme.
    pub fn resolve(&self, theme: &MechanixTheme) -> ResolvedTextStyle {
        let color = self.color_variant.resolve(&theme.colors);
        let type_style: &TypographyStyle = self.typography_variant.resolve(&theme.typography);

        let weight = self.weight.unwrap_or_else(|| {
            if self.is_emphasised {
                type_style.weight_emphasised
            } else {
                type_style.weight
            }
        });

        ResolvedTextStyle {
            color: self.color_override.unwrap_or(color),
            font_size: type_style.font_size,
            line_height: self.line_height.unwrap_or(type_style.line_height),
            letter_spacing: self.letter_spacing.unwrap_or(type_style.letter_spacing),
            word_spacing: self.word_spacing.unwrap_or(type_style.word_spacing),
            weight,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedTextStyle {
    /// Foreground color (resolved from `color_variant`).
    pub color: Color,
    /// Font size in scale-independent pixels.
    pub font_size: f32,
    /// Line height in pixels.
    pub line_height: f32,
    /// Letter spacing (tracking) in pixels.
    pub letter_spacing: f32,
    /// Word spacing in pixels.
    pub word_spacing: f32,
    /// Font weight enum (Thin 100, Regular 400, Medium 500, Bold 700, Black 900, etc.).
    pub weight: FontWeight,
}

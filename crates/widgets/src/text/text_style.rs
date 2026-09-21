use theme::{FontWeight, MechanixTheme, TextVariant, ThemeColor};

use utils::Color;

use crate::utils::ColorSource;

/// Unresolved Material 3 typography design tokens for a [`Text`](super::Text) widget.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextStyle {
    pub color: ColorSource,
    pub typography_variant: TextVariant,
    pub weight: Option<FontWeight>,
    pub is_emphasised: bool,
    pub line_height: Option<f32>,
    pub letter_spacing: Option<f32>,
    pub word_spacing: Option<f32>,
}

impl Default for TextStyle {
    /// Material 3 default: `OnSurface` color role, `BodyMedium` typography.
    fn default() -> Self {
        Self {
            color: ThemeColor::OnSurface.into(),
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
    pub fn resolve(&self, theme: &MechanixTheme) -> ResolvedTextStyle {
        let color = self.color.resolve(&theme.colors);
        let type_style = self.typography_variant.resolve(&theme.typography);

        let weight = self.weight.unwrap_or(if self.is_emphasised {
            type_style.weight_emphasised
        } else {
            type_style.weight
        });

        ResolvedTextStyle {
            color,
            font_size: type_style.font_size,
            line_height: self.line_height.unwrap_or(type_style.line_height),
            letter_spacing: self.letter_spacing.unwrap_or(type_style.letter_spacing),
            word_spacing: self.word_spacing.unwrap_or(0.0),
            weight,
        }
    }
}

/// Fully resolved, concrete typography style parameters ready for measurement & painting.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedTextStyle {
    /// Foreground color.
    pub color: Color,
    /// Font size in scale-independent pixels.
    pub font_size: f32,
    /// Line height in pixels.
    pub line_height: f32,
    /// Letter spacing in pixels.
    pub letter_spacing: f32,
    /// Word spacing in pixels.
    pub word_spacing: f32,
    /// Concrete font weight.
    pub weight: FontWeight,
}

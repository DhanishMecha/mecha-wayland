use crate::color::{ColorScheme, ColorVariant};
use utils::Color;

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
    #[inline]
    pub fn is_disabled(self) -> bool {
        matches!(self, WidgetState::Disabled)
    }

    /// Returns `true` if a state layer overlay should be applied.
    #[inline]
    pub fn has_state_layer(self) -> bool {
        matches!(
            self,
            WidgetState::Hovered | WidgetState::Focused | WidgetState::Pressed
        )
    }
}

/// M3 standard opacities:
/// - Hover: `0.08`
/// - Focus: `0.12`
/// - Press: `0.12`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StateLayer {
    /// The color variant whose resolved color is used as the overlay.
    pub color_variant: ColorVariant,
    /// Opacity of the overlay in [0, 1].
    pub opacity: f32,
}

impl StateLayer {
    pub fn apply(self, base_color: Color, scheme: &ColorScheme) -> Color {
        let overlay_rgb = self.color_variant.resolve(scheme);
        let overlay = Color::rgba(overlay_rgb.r, overlay_rgb.g, overlay_rgb.b, self.opacity);
        overlay.over(base_color)
    }
}

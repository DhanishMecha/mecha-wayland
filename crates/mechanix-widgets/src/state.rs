//! Widget interaction state and state-layer overlay types.

use geometry::Color;
use theme::{ColorRole, ColorScheme};

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

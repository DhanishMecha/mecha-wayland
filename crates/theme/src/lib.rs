pub mod color;
pub mod elevation;
pub mod shapes;
pub mod state;
pub mod typography;
pub mod utils;

pub use color::{ColorScheme, ColorVariant};
pub use elevation::{Elevation, ElevationScale};
pub use shapes::{Shape, Shapes};
pub use state::{StateLayer, WidgetState};
pub use typography::{FontWeight, TextVariant, Typography, TypographyStyle};
pub use utils::EdgeInsets;

use ::utils::Color;
use app::{App, Module, Signal};

// ThemeMode
/// Visual mode of the theme (`Light` or `Dark`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    Light,
    #[default]
    Dark,
}

// ── MechanixTheme ─────────────────────────────────────────────────────────

/// The Mechanix design system: colors, typography, shapes, and
/// elevation gathered into one app-wide [`Resource`](app::Resource).
///
/// Swap the whole theme at runtime and emit [`ThemeChanged`] to propagate the
/// update to all widgets.
#[derive(Debug, Clone)]
pub struct MechanixTheme {
    pub mode: ThemeMode,
    pub colors: ColorScheme,
    pub typography: Typography,
    pub shapes: Shapes,
    pub elevation: ElevationScale,
}

impl app::Resource for MechanixTheme {}

impl MechanixTheme {
    /// Returns `true` if the theme is in dark mode.
    #[inline]
    pub fn is_dark(&self) -> bool {
        self.mode == ThemeMode::Dark
    }
    /// Mechanix light theme.
    pub fn light() -> Self {
        Self {
            mode: ThemeMode::Light,
            colors: ColorScheme::baseline_light(),
            typography: Typography::default(),
            shapes: Shapes::default(),
            elevation: ElevationScale::default(),
        }
    }

    /// Mechanix dark theme.
    pub fn dark() -> Self {
        Self {
            mode: ThemeMode::Dark,
            colors: ColorScheme::baseline_dark(),
            typography: Typography::default(),
            shapes: Shapes::default(),
            elevation: ElevationScale::default(),
        }
    }

    /// Resolve a [`ColorVariant`] to a concrete `utils::Color`.
    #[inline]
    pub fn color(&self, role: ColorVariant) -> Color {
        role.resolve(&self.colors)
    }

    /// Resolve a [`TextVariant`] to a `&TypographyStyle`.
    #[inline]
    pub fn typography(&self, variant: TextVariant) -> &TypographyStyle {
        variant.resolve(&self.typography)
    }
}

impl Module for MechanixTheme {
    fn install(self, app: &mut App) {
        app.insert_resource(self);
        app.system(on_theme_changed);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemeChanged;
impl Signal for ThemeChanged {}

fn on_theme_changed(app: &mut App, _: &ThemeChanged) {
    app.emit_all(ApplyTheme);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplyTheme;
impl app::Event for ApplyTheme {}

pub mod prelude {
    pub use crate::*;
}

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

#[derive(Debug, Clone)]
pub struct MechanixTheme {
    pub colors: ColorScheme,
    pub typography: Typography,
    pub shapes: Shapes,
    pub elevation: ElevationScale,
}

impl MechanixTheme {
    /// Mechanix light theme.
    pub fn light() -> Self {
        let colors = ColorScheme::baseline_light();
        let typography = Typography::default();
        let shapes = Shapes::default();
        let elevation = ElevationScale::default();
        Self {
            colors,
            typography,
            shapes,
            elevation,
        }
    }

    /// Mechanix dark theme.
    pub fn dark() -> Self {
        let colors = ColorScheme::baseline_dark();
        let typography = Typography::default();
        let shapes = Shapes::default();
        let elevation = ElevationScale::default();
        Self {
            colors,
            typography,
            shapes,
            elevation,
        }
    }
}

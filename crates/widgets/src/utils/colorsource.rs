use theme::{ColorScheme, ThemeColor};
use utils::Color;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorSource {
    Exact(Color),
    Role(ThemeColor),
}

impl ColorSource {
    #[inline]
    pub fn resolve(self, scheme: &ColorScheme) -> Color {
        match self {
            Self::Exact(color) => color,
            Self::Role(variant) => variant.resolve(scheme),
        }
    }
}

impl From<Color> for ColorSource {
    #[inline]
    fn from(color: Color) -> Self {
        Self::Exact(color)
    }
}

impl From<ThemeColor> for ColorSource {
    #[inline]
    fn from(variant: ThemeColor) -> Self {
        Self::Role(variant)
    }
}

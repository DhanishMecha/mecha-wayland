use geometry::Color;
use theme::{ColorRole, ThemeReader};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorSource {
    Exact(Color),
    Role(ColorRole),
}

impl ColorSource {
    #[inline]
    pub fn resolve(self, reader: &impl ThemeReader) -> Color {
        match self {
            Self::Exact(color) => color,
            Self::Role(role) => reader.color(role),
        }
    }

    #[inline]
    pub fn is_theme_role(&self) -> bool {
        matches!(self, Self::Role(_))
    }
}

impl From<Color> for ColorSource {
    #[inline]
    fn from(color: Color) -> Self {
        Self::Exact(color)
    }
}

impl From<ColorRole> for ColorSource {
    #[inline]
    fn from(role: ColorRole) -> Self {
        Self::Role(role)
    }
}

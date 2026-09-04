#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Shape {
    /// 0 dp — sharp corners. Used for fullscreen or flush elements.
    #[default]
    None,
    /// Fully rounded pill — radius = `component_height / 2`.
    Full,
}

impl Shape {
    /// Convert this shape token to a corner radius in pixels.
    pub fn radius_px(self, component_height: f32) -> f32 {
        match self {
            Shape::None => 0.0,
            Shape::Full => component_height / 2.0,
        }
    }
}

// Shapes

/// Material 3 shape scale containing supported shape variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shapes {
    pub none: Shape,
    pub full: Shape,
}

impl Default for Shapes {
    fn default() -> Self {
        Self {
            none: Shape::None,
            full: Shape::Full,
        }
    }
}

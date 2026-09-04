use taffy::{LengthPercentage, Rect as TaffyRect};

/// Helper struct for constructing [`taffy::Rect<LengthPercentage>`] insets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdgeInsets;

impl EdgeInsets {
    /// Uniform padding for all four edges.
    pub const fn all(value: f32) -> TaffyRect<LengthPercentage> {
        TaffyRect {
            left: LengthPercentage::length(value),
            right: LengthPercentage::length(value),
            top: LengthPercentage::length(value),
            bottom: LengthPercentage::length(value),
        }
    }

    /// Symmetric padding: `horizontal` for left/right, `vertical` for top/bottom.
    pub const fn symmetric(horizontal: f32, vertical: f32) -> TaffyRect<LengthPercentage> {
        TaffyRect {
            left: LengthPercentage::length(horizontal),
            right: LengthPercentage::length(horizontal),
            top: LengthPercentage::length(vertical),
            bottom: LengthPercentage::length(vertical),
        }
    }

    /// Horizontal padding only (left and right).
    pub const fn horizontal(value: f32) -> TaffyRect<LengthPercentage> {
        Self::symmetric(value, 0.0)
    }

    /// Vertical padding only (top and bottom).
    pub const fn vertical(value: f32) -> TaffyRect<LengthPercentage> {
        Self::symmetric(0.0, value)
    }

    /// Explicit padding for each individual edge.
    pub const fn only(top: f32, right: f32, bottom: f32, left: f32) -> TaffyRect<LengthPercentage> {
        TaffyRect {
            left: LengthPercentage::length(left),
            right: LengthPercentage::length(right),
            top: LengthPercentage::length(top),
            bottom: LengthPercentage::length(bottom),
        }
    }

    /// Zero padding.
    pub const fn zero() -> TaffyRect<LengthPercentage> {
        Self::all(0.0)
    }
}

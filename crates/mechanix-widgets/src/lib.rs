//! The `mechanix-widgets` crate: higher-level, theme-aware widgets built on
//! top of the primitive `widgets` crate and `theme` crate.

pub mod color;
pub mod font;
mod text;

pub use color::ColorSource;
pub use font::{FontBook, FontContextExt};
pub use text::{Text, TextBuilder, TextContextExt, text};
pub use widgets::{TextAlign, TextDecoration, TextOverflow, TextWrap, VerticalTrim};

pub mod prelude {
    pub use crate::{
        ColorSource, FontBook, FontContextExt, Text, TextAlign, TextBuilder, TextContextExt,
        TextDecoration, TextOverflow, TextWrap, VerticalTrim, text,
    };
    pub use theme::{ColorRole, FontWeight, TextVariant};
}

//! The `mechanix-widgets` crate: higher-level, theme-aware widgets built on
//! top of the primitive `widgets` crate and `theme` crate.

pub mod button;
pub mod color;
pub mod font;
mod text;

pub use button::{
    Button, ButtonBuilder, ButtonContext, ButtonContextExt, ButtonOverrides, ButtonSize,
    ButtonStyle, ButtonVariant, OnClickHandler, ResolvedButtonStyle, SetButtonState, StateLayer,
    WidgetState, button,
};
pub use color::ColorSource;
pub use font::{FontBook, FontContextExt};
pub use text::{Text, TextBuilder, TextContextExt, text};
pub use widgets::{TextAlign, TextDecoration, TextOverflow, TextWrap, VerticalTrim};

pub mod prelude {
    pub use crate::button::{
        Button, ButtonBuilder, ButtonContext, ButtonContextExt, ButtonOverrides, ButtonSize,
        ButtonStyle, ButtonVariant, OnClickHandler, ResolvedButtonStyle, SetButtonState,
        StateLayer, WidgetState, button,
    };
    pub use crate::{
        ColorSource, FontBook, FontContextExt, Text, TextAlign, TextBuilder, TextContextExt,
        TextDecoration, TextOverflow, TextWrap, VerticalTrim, text,
    };
    pub use theme::{ColorRole, FontWeight, TextVariant};
}

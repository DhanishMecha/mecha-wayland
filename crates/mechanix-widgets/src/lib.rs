//! The `mechanix-widgets` crate: higher-level, theme-aware widgets built on
//! top of the primitive `widgets` crate and `theme` crate.

pub mod button;
pub mod color;
pub mod font;
pub mod mechanix_button;
pub mod state;
mod text;

pub use button::{
    Button, ButtonBuilder, ButtonContext, ButtonContextExt, ButtonStateChanged, ChildFn,
    OnClickHandler, SetButtonState, button,
};
pub use color::ColorSource;
pub use font::{FontBook, FontContextExt};
pub use mechanix_button::{
    ButtonOverrides, ButtonSize, ButtonStyle, ButtonVariant, MechanixButton,
    MechanixButtonBuilder, MechanixButtonContext, MechanixButtonContextExt,
    MechanixButtonOverrides, MechanixButtonSize, MechanixButtonStyle, MechanixButtonVariant,
    ResolvedButtonStyle, ResolvedMechanixButtonStyle, mechanix_button,
};
pub use state::{StateLayer, WidgetState};
pub use text::{Text, TextBuilder, TextContextExt, text};
pub use widgets::{TextAlign, TextDecoration, TextOverflow, TextWrap, VerticalTrim};

pub mod prelude {
    pub use crate::button::{
        Button, ButtonBuilder, ButtonContext, ButtonContextExt, ButtonStateChanged, ChildFn,
        OnClickHandler, SetButtonState, button,
    };
    pub use crate::mechanix_button::{
        ButtonOverrides, ButtonSize, ButtonStyle, ButtonVariant, MechanixButton,
        MechanixButtonBuilder, MechanixButtonContext, MechanixButtonContextExt,
        MechanixButtonOverrides, MechanixButtonSize, MechanixButtonStyle, MechanixButtonVariant,
        ResolvedButtonStyle, ResolvedMechanixButtonStyle, mechanix_button,
    };
    pub use crate::state::{StateLayer, WidgetState};
    pub use crate::{
        ColorSource, FontBook, FontContextExt, Text, TextAlign, TextBuilder, TextContextExt,
        TextDecoration, TextOverflow, TextWrap, VerticalTrim, text,
    };
    pub use theme::{ColorRole, FontWeight, TextVariant};
}

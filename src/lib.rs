//! The `mecha-wayland` facade: one crate to depend on, one prelude to import.
//! Each member crate is re-exported by name, and its prelude is folded into
//! [`prelude`].

pub use app;
pub use assets;
pub use layout;
pub use paint;
pub use utils;
pub use widgets;
pub use window;

pub mod prelude {
    pub use app::prelude::*;
    pub use layout::prelude::*;
    pub use paint::prelude::*;
    pub use utils::prelude::*;
    pub use widgets::prelude::*;
    pub use window::prelude::*;
}

//! The `mecha-wayland` facade: one crate to depend on, one prelude to import.
//! Each member crate is re-exported by name, and its prelude is folded into
//! [`prelude`].
//!
//! A windowed app installs, in this order: `RingModule`, `WaylandModule`,
//! `LayoutModule`, `PaintModule`, `WindowModule`, `InteractivityModule`,
//! `PresentationModule`, `RenderModule`. Each builds on the ones before it;
//! `examples/counter.rs` is the shape of it.

pub use app;
pub use assets;
pub use interactivity;
pub use layout;
pub use paint;
pub use presentation;
pub use renderer;
pub use ring;
pub use theme;
pub use wayland;
pub use widgets;
pub use window;

pub mod prelude {
    pub use app::prelude::*;
    pub use interactivity::prelude::*;
    pub use layout::prelude::*;
    pub use paint::prelude::*;
    pub use presentation::prelude::*;
    pub use renderer::prelude::*;
    pub use ring::prelude::*;
    pub use theme::prelude::*;
    pub use wayland::prelude::*;
    pub use widgets::prelude::*;
    pub use window::prelude::*;
}

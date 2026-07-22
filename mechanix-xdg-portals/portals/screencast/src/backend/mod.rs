pub mod backend;
pub mod interface;
pub mod types;

pub use backend::{screencast_backend_module, ScreenCastBackend};
pub use interface::{ScreenCastIface, SCREENCAST_IFACE, SCREENCAST_VERSION};
pub use types::*;

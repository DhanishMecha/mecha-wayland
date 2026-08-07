pub mod interface;
pub mod types;
mod backend;

pub use backend::{clipboard_backend_module, ClipboardBackend};
pub use interface::ClipboardIface;

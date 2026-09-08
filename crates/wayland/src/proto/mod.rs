#[allow(unused_variables, unused_mut, dead_code, unused_imports, clippy::all)]
pub mod generated;
pub mod manual;

pub use generated::*;
pub use manual::client::{WlCallbackEvent, WlDisplayError, WlDisplayEvent, WlRegistryEvent};
pub use manual::{WlCallback, WlDisplay, WlRegistry};

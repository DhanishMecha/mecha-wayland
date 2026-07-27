pub mod interface;
pub mod types;
mod backend;
pub mod watcher;
pub mod helpers;

pub use backend::{settings_backend_module, SettingsBackend};
pub use interface::SettingsIface;
pub use types::{
    AccentColor, ColorScheme, Contrast, ReducedMotion,
    APPEARANCE_NS, KEY_ACCENT_COLOR, KEY_COLOR_SCHEME, KEY_CONTRAST, KEY_REDUCED_MOTION,
};

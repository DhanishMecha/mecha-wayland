pub mod backend;
pub mod interface;
pub mod types;

pub use backend::{wallpaper_backend_module, WallpaperBackend};
pub use interface::{WallpaperIface, WALLPAPER_IFACE, WALLPAPER_VERSION};

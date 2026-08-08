use dbus::dbus_interface;
use zbus::zvariant::OwnedObjectPath;

use super::types::WallpaperOptions;

pub const WALLPAPER_IFACE: &str = "org.freedesktop.impl.portal.Wallpaper";
pub const WALLPAPER_VERSION: u32 = 1;

dbus_interface!(pub WallpaperIface = WALLPAPER_IFACE;
    method SetWallpaperURI(
        handle: OwnedObjectPath,
        app_id: String,
        parent_window: String,
        uri: String,
        options: WallpaperOptions
    ) -> (response: u32);
    property version: u32, read;
);

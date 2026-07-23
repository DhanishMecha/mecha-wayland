use dbus::dbus_interface;
use zbus::zvariant::OwnedObjectPath;

use super::types::{ FileChooserResults, OpenFileOptions, SaveFileOptions, SaveFilesOptions };

pub const FILECHOOSER_IFACE: &str = "org.freedesktop.impl.portal.FileChooser";
pub const FILECHOOSER_VERSION: u32 = 3;

// XDG Desktop Portal FileChooser interface:
// https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.impl.portal.FileChooser.html
dbus_interface!(pub FileChooser = FILECHOOSER_IFACE;
    method OpenFile(
        handle: OwnedObjectPath, app_id: String, parent_window: String,
        title: String, options: OpenFileOptions
    ) -> (response: u32, results: FileChooserResults);
    method SaveFile(
        handle: OwnedObjectPath, app_id: String, parent_window: String,
        title: String, options: SaveFileOptions
    ) -> (response: u32, results: FileChooserResults);
    method SaveFiles(
        handle: OwnedObjectPath, app_id: String, parent_window: String,
        title: String, options: SaveFilesOptions
    ) -> (response: u32, results: FileChooserResults);
    property version: u32, read;
);

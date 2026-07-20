use dbus::dbus_interface;
use zbus::zvariant::OwnedObjectPath;

use super::types::{ PickColorOptions, PickColorResults, ScreenshotOptions, ScreenshotResults };

pub const SCREENSHOT_IFACE: &str = "org.freedesktop.impl.portal.Screenshot";
pub const SCREENSHOT_VERSION: u32 = 3;

dbus_interface!(pub ScreenshotIface = SCREENSHOT_IFACE;
    method Screenshot(
        handle: OwnedObjectPath,
        app_id: String,
        parent_window: String,
        options: ScreenshotOptions
    ) -> (response: u32, results: ScreenshotResults);
    method PickColor(
        handle: OwnedObjectPath,
        app_id: String,
        parent_window: String,
        options: PickColorOptions
    ) -> (response: u32, results: PickColorResults);
    property version: u32, read;
);

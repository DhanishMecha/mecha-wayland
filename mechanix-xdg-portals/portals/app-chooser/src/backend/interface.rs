use dbus::dbus_interface;
use zbus::zvariant::OwnedObjectPath;

use super::types::{AppChooserOptions, AppChooserResults};

pub const APP_CHOOSER_IFACE: &str = "org.freedesktop.impl.portal.AppChooser";
pub const APP_CHOOSER_VERSION: u32 = 2;

dbus_interface!(pub AppChooserIface = APP_CHOOSER_IFACE;
    method ChooseApplication(
        handle: OwnedObjectPath,
        app_id: String,
        parent_window: String,
        choices: Vec<String>,
        options: AppChooserOptions
    ) -> (response: u32, results: AppChooserResults);
    method UpdateChoices(
        handle: OwnedObjectPath,
        choices: Vec<String>
    ) -> ();
    property version: u32, read;
);

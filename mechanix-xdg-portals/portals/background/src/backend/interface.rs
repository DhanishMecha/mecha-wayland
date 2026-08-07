use dbus::dbus_interface;
use zbus::zvariant::{OwnedObjectPath, OwnedValue};
use std::collections::HashMap;

pub const BACKGROUND_IFACE: &str = "org.freedesktop.impl.portal.Background";
pub const BACKGROUND_VERSION: u32 = 1;

dbus_interface!(pub BackgroundIface = BACKGROUND_IFACE;
    method GetAppState() -> (apps: HashMap<String, OwnedValue>);
    method NotifyBackground(
        handle: OwnedObjectPath,
        app_id: String,
        name: String,
    ) -> (response: u32, results: HashMap<String, OwnedValue>);
    method EnableAutostart(
        app_id: String,
        enable: bool,
        commandline: Vec<String>,
        flags: u32,
    ) -> (result: bool);
    signal RunningApplicationsChanged();
);

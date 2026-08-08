use dbus::dbus_interface;
use std::collections::HashMap;
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

pub const DYNAMIC_LAUNCHER_IFACE: &str = "org.freedesktop.impl.portal.DynamicLauncher";
pub const DYNAMIC_LAUNCHER_VERSION: u32 = 1;

dbus_interface!(pub DynamicLauncherIface = DYNAMIC_LAUNCHER_IFACE;

    method PrepareInstall(
        handle: OwnedObjectPath,
        app_id: String,
        parent_window: String,
        name: String,
        icon_v: OwnedValue,
        options: HashMap<String, OwnedValue>,
    ) -> (response: u32, results: HashMap<String, OwnedValue>);

    method RequestInstallToken(
        app_id: String,
        options: HashMap<String, OwnedValue>,
    ) -> (response: u32);
);

use dbus::dbus_interface;
use zbus::zvariant::{OwnedFd, OwnedObjectPath, OwnedValue};
use std::collections::HashMap;

pub const PRINT_IFACE: &str = "org.freedesktop.impl.portal.Print";
pub const PRINT_VERSION: u32 = 1;

dbus_interface!(pub PrintIface = PRINT_IFACE;
    method PreparePrint(
        handle: OwnedObjectPath,
        app_id: String,
        parent_window: String,
        title: String,
        settings: HashMap<String, OwnedValue>,
        page_setup: HashMap<String, OwnedValue>,
        options: HashMap<String, OwnedValue>,
    ) -> (response: u32, results: HashMap<String, OwnedValue>);

    method Print(
        handle: OwnedObjectPath,
        app_id: String,
        parent_window: String,
        title: String,
        fd: OwnedFd,
        options: HashMap<String, OwnedValue>,
    ) -> (response: u32, results: HashMap<String, OwnedValue>);

    property version: u32, read;
);

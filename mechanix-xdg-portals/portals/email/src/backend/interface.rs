use dbus::dbus_interface;
use std::collections::HashMap;
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

pub const EMAIL_IFACE: &str = "org.freedesktop.impl.portal.Email";
pub const EMAIL_VERSION: u32 = 4;

dbus_interface!(pub EmailIface = EMAIL_IFACE;
    method ComposeEmail(
        handle: OwnedObjectPath,
        app_id: String,
        parent_window: String,
        options: HashMap<String, OwnedValue>,
    ) -> (response: u32, results: HashMap<String, OwnedValue>);
    property version: u32, read;
);

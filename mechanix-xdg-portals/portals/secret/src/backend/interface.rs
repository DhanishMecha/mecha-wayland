use dbus::dbus_interface;
use zbus::zvariant::{OwnedFd, OwnedObjectPath, OwnedValue};
use std::collections::HashMap;

pub const SECRET_IFACE: &str = "org.freedesktop.impl.portal.Secret";
pub const SECRET_VERSION: u32 = 1;

dbus_interface!(pub SecretIface = SECRET_IFACE;
    method RetrieveSecret(
        handle: OwnedObjectPath,
        app_id: String,
        fd: OwnedFd,
        options: HashMap<String, OwnedValue>,
    ) -> (response: u32, results: HashMap<String, OwnedValue>);
    property version: u32, read;
);

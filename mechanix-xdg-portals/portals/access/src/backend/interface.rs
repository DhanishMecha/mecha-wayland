use dbus::dbus_interface;
use zbus::zvariant::OwnedObjectPath;

use super::types::{AccessOptions, AccessResults};

pub const ACCESS_IFACE: &str = "org.freedesktop.impl.portal.Access";
pub const ACCESS_VERSION: u32 = 1;

dbus_interface!(pub Access = ACCESS_IFACE;
    method AccessDialog(
        handle: OwnedObjectPath,
        app_id: String,
        parent_window: String,
        title: String,
        subtitle: String,
        body: String,
        options: AccessOptions
    ) -> (response: u32, results: AccessResults);
    property version: u32, read;
);

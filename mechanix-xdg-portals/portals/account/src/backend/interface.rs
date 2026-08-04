use dbus::{dbus_interface, dbus_method};
use zbus::zvariant::{OwnedObjectPath, OwnedValue};
use std::collections::HashMap;

use super::types::{AccountOptions, AccountResults};

pub const ACCOUNT_IFACE: &str = "org.freedesktop.impl.portal.Account";
pub const ACCOUNT_VERSION: u32 = 1;

dbus_interface!(pub AccountIface = ACCOUNT_IFACE;
    // Returns basic user information: name, id, and avatar image URI.
    // The reply is deferred until the user grants or denies the consent dialog.
    method GetUserInformation(
        handle: OwnedObjectPath,
        app_id: String,
        parent_window: String,
        options: AccountOptions,
    ) -> (response: u32, results: AccountResults);
);

dbus_method!(pub FindUserById {
    dest: "org.freedesktop.Accounts",
    path: "/org/freedesktop/Accounts",
    iface: "org.freedesktop.Accounts",
    member: "FindUserById",
    args: (i64,),
    reply: OwnedObjectPath,
});

dbus_method!(pub GetUserProperties {
    dest: "org.freedesktop.Accounts",
    path: "/org/freedesktop/Accounts", // Default fallback path, overridden by call_at
    iface: "org.freedesktop.DBus.Properties",
    member: "GetAll",
    args: (String,),
    reply: HashMap<String, OwnedValue>,
});

use dbus::dbus_interface;
use std::collections::HashMap;
use zbus::zvariant::OwnedValue;

pub const PERMISSION_STORE_IFACE: &str = "org.freedesktop.impl.portal.PermissionStore";
pub const PERMISSION_STORE_VERSION: u32 = 2;
pub const PERMISSION_STORE_PATH: &str = "/org/freedesktop/impl/portal/PermissionStore";

dbus_interface!(pub PermissionStoreIface = PERMISSION_STORE_IFACE;
    // Looks up the permissions and associated data for a specific resource ID in a table.
    method Lookup(
        table: String,
        id: String
    ) -> (
        permissions: HashMap<String, Vec<String>>,
        data: OwnedValue
    );

    // Creates or updates an entry for a resource ID with the given permissions and data.
    method Set(
        table: String,
        create: bool,
        id: String,
        app_permissions: HashMap<String, Vec<String>>,
        data: OwnedValue
    ) -> ();

    // Deletes the entry for a resource ID from a table.
    method Delete(
        table: String,
        id: String
    ) -> ();

    // Updates only the associated data for a resource ID in a table.
    method SetValue(
        table: String,
        create: bool,
        id: String,
        data: OwnedValue
    ) -> ();

    // Sets permissions for a specific application for a resource ID.
    method SetPermission(
        table: String,
        create: bool,
        id: String,
        app_id: String,
        permissions: Vec<String>
    ) -> ();

    // Deletes permissions for a specific application for a resource ID.
    method DeletePermission(
        table: String,
        id: String,
        app_id: String
    ) -> ();

    // Retrieves permissions for a specific application for a resource ID.
    method GetPermission(
        table: String,
        id: String,
        app_id: String
    ) -> (
        permissions: Vec<String>
    );

    // Lists all resource IDs present in a table.
    method List(
        table: String
    ) -> (
        ids: Vec<String>
    );

    // Emitted when a permission store entry is modified or deleted.
    signal Changed(
        table: String,
        id: String,
        deleted: bool,
        data: OwnedValue,
        permissions: HashMap<String, Vec<String>>
    );

    // The version of the PermissionStore interface.
    property version: u32, read;
);

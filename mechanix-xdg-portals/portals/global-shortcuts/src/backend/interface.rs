use dbus::dbus_interface;
use zbus::zvariant::{OwnedObjectPath, OwnedValue};
use std::collections::HashMap;

pub const GLOBAL_SHORTCUTS_IFACE: &str = "org.freedesktop.impl.portal.GlobalShortcuts";
pub const GLOBAL_SHORTCUTS_VERSION: u32 = 2;

dbus_interface!(pub GlobalShortcutsIface = GLOBAL_SHORTCUTS_IFACE;
    method CreateSession(
        handle: OwnedObjectPath,
        session_handle: OwnedObjectPath,
        app_id: String,
        options: HashMap<String, OwnedValue>
    ) -> (response: u32, results: HashMap<String, OwnedValue>);

    method BindShortcuts(
        handle: OwnedObjectPath,
        session_handle: OwnedObjectPath,
        shortcuts: Vec<(String, HashMap<String, OwnedValue>)>,
        parent_window: String,
        options: HashMap<String, OwnedValue>
    ) -> (response: u32, results: HashMap<String, OwnedValue>);

    method ListShortcuts(
        handle: OwnedObjectPath,
        session_handle: OwnedObjectPath
    ) -> (response: u32, results: HashMap<String, OwnedValue>);

    method ConfigureShortcuts(
        session_handle: OwnedObjectPath,
        parent_window: String,
        options: HashMap<String, OwnedValue>
    ) -> ();

    signal Activated(
        session_handle: OwnedObjectPath,
        shortcut_id: String,
        timestamp: u64,
        options: HashMap<String, OwnedValue>
    );

    signal Deactivated(
        session_handle: OwnedObjectPath,
        shortcut_id: String,
        timestamp: u64,
        options: HashMap<String, OwnedValue>
    );

    signal ShortcutsChanged(
        session_handle: OwnedObjectPath,
        shortcuts: Vec<(String, HashMap<String, OwnedValue>)>
    );

    property version: u32, read;
);

use dbus::dbus_interface;
use zbus::zvariant::OwnedValue;
use std::collections::HashMap;

pub const SETTINGS_IFACE: &str = "org.freedesktop.impl.portal.Settings";
pub const SETTINGS_VERSION: u32 = 2;

dbus_interface!(pub SettingsIface = SETTINGS_IFACE;
    method ReadAll(namespaces: Vec<String>) -> (settings: HashMap<String, HashMap<String, OwnedValue>>);
    method Read(namespace: String, key: String) -> (value: OwnedValue);
    signal SettingChanged(namespace: String, key: String, value: OwnedValue);
    property version: u32, read;
);

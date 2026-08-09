use dbus::dbus_interface;
use zbus::zvariant::OwnedObjectPath;
use std::collections::HashMap;
use zbus::zvariant::OwnedValue;

pub const USB_IFACE: &str = "org.freedesktop.impl.portal.Usb";
pub const USB_VERSION: u32 = 1;

dbus_interface!(pub UsbIface = USB_IFACE;
    method AcquireDevices(
        handle: OwnedObjectPath,
        parent_window: String,
        app_id: String,
        devices: Vec<(String, HashMap<String, OwnedValue>, HashMap<String, OwnedValue>)>,
        options: HashMap<String, OwnedValue>
    ) -> (response: u32, results: HashMap<String, OwnedValue>);
    property version: u32, read;
);


use dbus::dbus_interface;
use std::collections::HashMap;
use zbus::zvariant::{OwnedFd, OwnedObjectPath, OwnedValue};

pub const INPUTCAPTURE_IFACE: &str = "org.freedesktop.impl.portal.InputCapture";
pub const INPUTCAPTURE_VERSION: u32 = 2;

dbus_interface!(pub InputCaptureIface = INPUTCAPTURE_IFACE;
    method CreateSession(
        handle: OwnedObjectPath,
        session_handle: OwnedObjectPath,
        app_id: String,
        parent_window: String,
        options: HashMap<String, OwnedValue>
    ) -> (response: u32, results: HashMap<String, OwnedValue>);

    method CreateSession2(
        session_handle: OwnedObjectPath,
        app_id: String,
        options: HashMap<String, OwnedValue> // no supported keys in the vardict
    ) -> (results: HashMap<String, OwnedValue>);

    method Start(
        handle: OwnedObjectPath,
        session_handle: OwnedObjectPath,
        app_id: String,
        parent_window: String,
        options: HashMap<String, OwnedValue>
    ) -> (response: u32, results: HashMap<String, OwnedValue>);

    method GetZones(
        handle: OwnedObjectPath,
        session_handle: OwnedObjectPath,
        app_id: String,
        options: HashMap<String, OwnedValue>
    ) -> (response: u32, results: HashMap<String, OwnedValue>);

    method SetPointerBarriers(
        handle: OwnedObjectPath,
        session_handle: OwnedObjectPath,
        app_id: String,
        options: HashMap<String, OwnedValue>,
        barriers: Vec<HashMap<String, OwnedValue>>,
        zone_set: u32
    ) -> (response: u32, results: HashMap<String, OwnedValue>);

    method Enable(
        session_handle: OwnedObjectPath,
        app_id: String,
        options: HashMap<String, OwnedValue>
    ) -> (response: u32, results: HashMap<String, OwnedValue>);

    method Disable(
        session_handle: OwnedObjectPath,
        app_id: String,
        options: HashMap<String, OwnedValue>
    ) -> (response: u32, results: HashMap<String, OwnedValue>);

    method Release(
        session_handle: OwnedObjectPath,
        app_id: String,
        options: HashMap<String, OwnedValue>
    ) -> (response: u32, results: HashMap<String, OwnedValue>);

    method ConnectToEIS(
        session_handle: OwnedObjectPath,
        app_id: String,
        options: HashMap<String, OwnedValue>
    ) -> (fd: OwnedFd);

    signal Disabled(
        session_handle: OwnedObjectPath,
        options: HashMap<String, OwnedValue>
    );

    signal Activated(
        session_handle: OwnedObjectPath,
        options: HashMap<String, OwnedValue>
    );

    signal Deactivated(
        session_handle: OwnedObjectPath,
        options: HashMap<String, OwnedValue>
    );

    signal ZonesChanged(
        session_handle: OwnedObjectPath,
        options: HashMap<String, OwnedValue>
    );

    property SupportedCapabilities: u32, read;
    property version: u32, read;
);

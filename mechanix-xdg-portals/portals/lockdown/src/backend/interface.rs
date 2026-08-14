use dbus::dbus_interface;

pub const LOCKDOWN_IFACE: &str = "org.freedesktop.impl.portal.Lockdown";
pub const LOCKDOWN_VERSION: u32 = 1;

dbus_interface!(pub LockdownIface = LOCKDOWN_IFACE;
    property version: u32, read;
    property disable_printing: bool, readwrite;
    property disable_save_to_disk: bool, readwrite;
    property disable_application_handlers: bool, readwrite;
    property disable_location: bool, readwrite;
    property disable_camera: bool, readwrite;
    property disable_microphone: bool, readwrite;
    property disable_sound_output: bool, readwrite;
);

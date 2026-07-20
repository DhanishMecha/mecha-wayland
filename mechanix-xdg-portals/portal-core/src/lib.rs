use dbus::dbus_handler;

// --- Shared portal constants ------------------------------------------------

pub const PORTAL_NAME: &str = "org.freedesktop.impl.portal.desktop.mechanix";
pub const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";
pub const REQUEST_IFACE: &str = "org.freedesktop.impl.portal.Request";

// Portal response codes (the `response u` out-arg used by all portals).
pub const RESPONSE_SUCCESS: u32 = 0;
pub const RESPONSE_CANCELLED: u32 = 1;
pub const RESPONSE_ENDED: u32 = 2;

// Reconnect retry interval in frames (~4 seconds at 60fps).
pub const RECONNECT_INTERVAL_FRAMES: u32 = 240;

// Every portal can receive a Close call on its Request object.
dbus_handler!(pub RequestClose {
    iface: REQUEST_IFACE,
    member: "Close",
    args: (),
    ret: ()
});

// --- Shared Connection Monitor -----------------------------------------------

pub mod dbus_monitor;
pub use dbus_monitor::{dbus_monitor_module, DbusMonitor};

pub mod portal_host;
pub use portal_host::{portal_host_module, PortalHost};

// --- Shared UI modules -------------------------------------------------------

pub mod atlas {
    include!(concat!(env!("OUT_DIR"), "/ui_gen.rs"));
}

pub mod widgets;

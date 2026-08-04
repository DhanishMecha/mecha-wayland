use dbus::{dbus_interface, dbus_method, dbus_signal};
use std::collections::HashMap;
use zbus::zvariant::{OwnedFd, OwnedObjectPath, OwnedValue};

pub const INHIBIT_IFACE: &str = "org.freedesktop.impl.portal.Inhibit";
pub const INHIBIT_VERSION: u32 = 3;

// Bitmask flags: 1=Logout, 2=UserSwitch, 4=Suspend, 8=Idle
pub const FLAG_LOGOUT: u32 = 1;
pub const FLAG_USER_SWITCH: u32 = 2;
pub const FLAG_SUSPEND: u32 = 4;
pub const FLAG_IDLE: u32 = 8;

dbus_interface!(pub InhibitIface = INHIBIT_IFACE;
    // Inhibit session state changes. No response out-args per spec —
    // inhibition ends when Request.Close is called on `handle`.
    method Inhibit(
        handle: OwnedObjectPath,
        app_id: String,
        parent_window: String,
        flags: u32,
        options: HashMap<String, OwnedValue>,
    ) -> ();

    // Creates a monitoring session. Returns response code (0 = success).
    // While the session is alive, StateChanged signals are sent to it.
    method CreateMonitor(
        handle: OwnedObjectPath,
        session_handle: OwnedObjectPath,
        app_id: String,
        parent_window: String,
    ) -> (response: u32);

    // Acknowledge a StateChanged signal with session-state = QueryEnd (2).
    // Must be called within ~1 second of receiving the signal.
    method QueryEndResponse(session_handle: OwnedObjectPath) -> ();

    // Helper for manual simulation/testing of state changes
    method SimulateStateChanged(state: u32, screensaver_active: bool) -> ();

    // Emitted to active monitoring sessions when session state changes.
    signal StateChanged(session_handle: OwnedObjectPath, state: HashMap<String, OwnedValue>);

);

dbus_method!(pub LogindInhibit {
    dest: "org.freedesktop.login1",
    path: "/org/freedesktop/login1",
    iface: "org.freedesktop.login1.Manager",
    member: "Inhibit",
    args: (String, String, String, String),
    reply: OwnedFd,
});

dbus_signal!(pub PrepareForSleep {
    iface: "org.freedesktop.login1.Manager",
    member: "PrepareForSleep",
    args: (bool,),
});

dbus_signal!(pub LockSession {
    iface: "org.freedesktop.login1.Session",
    member: "Lock",
    args: (),
});

dbus_signal!(pub UnlockSession {
    iface: "org.freedesktop.login1.Session",
    member: "Unlock",
    args: (),
});


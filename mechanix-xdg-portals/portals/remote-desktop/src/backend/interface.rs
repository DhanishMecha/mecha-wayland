use dbus::dbus_interface;
use zbus::zvariant::{OwnedFd, OwnedObjectPath};

use super::types::{
    ConnectToEISOptions, CreateSessionOptions, CreateSessionResults, NotifyKeyboardKeycodeOptions,
    NotifyKeyboardKeysymOptions, NotifyPointerAxisDiscreteOptions, NotifyPointerAxisOptions,
    NotifyPointerButtonOptions, NotifyPointerMotionAbsoluteOptions, NotifyPointerMotionOptions,
    NotifyTouchDownOptions, NotifyTouchMotionOptions, NotifyTouchUpOptions, SelectDevicesOptions,
    SelectDevicesResults, StartOptions, StartResults,
};

pub const REMOTE_DESKTOP_IFACE: &str = "org.freedesktop.impl.portal.RemoteDesktop";
pub const REMOTE_DESKTOP_VERSION: u32 = 2;

// Device type bitmask constants (AvailableDeviceTypes property):
//   KEYBOARD     = 1
//   POINTER      = 2
//   TOUCHSCREEN  = 4

dbus_interface!(pub RemoteDesktopIface = REMOTE_DESKTOP_IFACE;
    // Creates a remote desktop session. No user interaction at this stage.
    method CreateSession(
        handle: OwnedObjectPath,
        session_handle: OwnedObjectPath,
        app_id: String,
        options: CreateSessionOptions,
    ) -> (response: u32, results: CreateSessionResults);

    // Configure which input devices (keyboard/pointer/touchscreen) are exposed.
    // options keys: types (u), restore_data ((suv)) [v2], persist_mode (u) [v2]
    method SelectDevices(
        handle: OwnedObjectPath,
        session_handle: OwnedObjectPath,
        app_id: String,
        options: SelectDevicesOptions,
    ) -> (response: u32, results: SelectDevicesResults);

    // Starts the remote desktop session; typically presents a dialog.
    // results keys: devices (u), clipboard_enabled (b) [v2],
    //               streams (a(ua{sv})), restore_data ((suv)) [v2]
    method Start(
        handle: OwnedObjectPath,
        session_handle: OwnedObjectPath,
        app_id: String,
        parent_window: String,
        options: StartOptions,
    ) -> (response: u32, results: StartResults);

    // NotifyPointerMotion(session_handle, options, dx, dy)
    // Relative pointer motion; requires POINTER access.
    method NotifyPointerMotion(
        session_handle: OwnedObjectPath,
        options: NotifyPointerMotionOptions,
        dx: f64,
        dy: f64,
    ) -> ();

    // NotifyPointerMotionAbsolute(session_handle, options, stream, x, y)
    // Absolute pointer motion in stream logical coordinate space; requires POINTER access.
    method NotifyPointerMotionAbsolute(
        session_handle: OwnedObjectPath,
        options: NotifyPointerMotionAbsoluteOptions,
        stream: u32,
        x: f64,
        y: f64,
    ) -> ();

    // NotifyPointerButton(session_handle, options, button, state)
    // Pointer button event; state: 0=released, 1=pressed. Requires POINTER access.
    method NotifyPointerButton(
        session_handle: OwnedObjectPath,
        options: NotifyPointerButtonOptions,
        button: i32,
        state: u32,
    ) -> ();

    // NotifyPointerAxis(session_handle, options, dx, dy)
    // Smooth scroll axis event (e.g. touchpad); options key: finish (b).
    // Requires POINTER access.
    method NotifyPointerAxis(
        session_handle: OwnedObjectPath,
        options: NotifyPointerAxisOptions,
        dx: f64,
        dy: f64,
    ) -> ();

    // NotifyPointerAxisDiscrete(session_handle, options, axis, steps)
    // Discrete scroll event; axis: 0=vertical, 1=horizontal. Requires POINTER access.
    method NotifyPointerAxisDiscrete(
        session_handle: OwnedObjectPath,
        options: NotifyPointerAxisDiscreteOptions,
        axis: u32,
        steps: i32,
    ) -> ();

    // NotifyKeyboardKeycode(session_handle, options, keycode, state)
    // Keyboard keycode event; state: 0=released, 1=pressed. Requires KEYBOARD access.
    method NotifyKeyboardKeycode(
        session_handle: OwnedObjectPath,
        options: NotifyKeyboardKeycodeOptions,
        keycode: i32,
        state: u32,
    ) -> ();

    // NotifyKeyboardKeysym(session_handle, options, keysym, state)
    // Keyboard keysym event; state: 0=released, 1=pressed. Requires KEYBOARD access.
    method NotifyKeyboardKeysym(
        session_handle: OwnedObjectPath,
        options: NotifyKeyboardKeysymOptions,
        keysym: i32,
        state: u32,
    ) -> ();

    // NotifyTouchDown(session_handle, options, stream, slot, x, y)
    // New touch-down event in stream logical coordinate space. Requires TOUCHSCREEN access.
    method NotifyTouchDown(
        session_handle: OwnedObjectPath,
        options: NotifyTouchDownOptions,
        stream: u32,
        slot: u32,
        x: f64,
        y: f64,
    ) -> ();

    // NotifyTouchMotion(session_handle, options, stream, slot, x, y)
    // Touch point motion in stream logical coordinate space. Requires TOUCHSCREEN access.
    method NotifyTouchMotion(
        session_handle: OwnedObjectPath,
        options: NotifyTouchMotionOptions,
        stream: u32,
        slot: u32,
        x: f64,
        y: f64,
    ) -> ();

    // NotifyTouchUp(session_handle, options, slot)
    // Touch-up event. Requires TOUCHSCREEN access.
    method NotifyTouchUp(
        session_handle: OwnedObjectPath,
        options: NotifyTouchUpOptions,
        slot: u32,
    ) -> ();

    // ConnectToEIS(session_handle, app_id, options) -> fd
    // Returns a file descriptor to an EIS socket. Preferred over Notify* methods.
    // Available in version 2 of this interface.
    method ConnectToEIS(
        session_handle: OwnedObjectPath,
        app_id: String,
        options: ConnectToEISOptions,
    ) -> (fd: OwnedFd);

    // AvailableDeviceTypes: bitmask of supported device types (KEYBOARD=1, POINTER=2, TOUCHSCREEN=4)
    property AvailableDeviceTypes: u32, read;

    property version: u32, read;
);

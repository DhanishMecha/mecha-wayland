use dbus::dbus_interface;
use std::collections::HashMap;
use zbus::zvariant::{OwnedFd, OwnedObjectPath, OwnedValue};

pub const CLIPBOARD_IFACE: &str = "org.freedesktop.impl.portal.Clipboard";
pub const CLIPBOARD_VERSION: u32 = 1;

dbus_interface!(pub ClipboardIface = CLIPBOARD_IFACE;

    // Attach clipboard access to an existing session (e.g. RemoteDesktop/InputCapture).
    // Must be called before the session is started.
    // options: a{sv} (currently no defined keys)
    method RequestClipboard(
        session_handle: OwnedObjectPath,
        options: HashMap<String, OwnedValue>,
    ) -> ();

    // Session advertises that it owns the current clipboard selection with the given MIME types.
    // options keys:
    //   "mime_types": as  — list of MIME types this session offers on the clipboard
    method SetSelection(
        session_handle: OwnedObjectPath,
        options: HashMap<String, OwnedValue>,
    ) -> ();

    // Answer to a SelectionTransfer signal. The session that owns the clipboard delivers
    // its data via a file descriptor to the method callee.
    // The callee (compositor) creates the file descriptor.
    // serial: matches the serial from the SelectionTransfer signal
    // fd (out): FD to write clipboard content into
    method SelectionWrite(
        session_handle: OwnedObjectPath,
        serial: u32,
    ) -> (fd: OwnedFd);

    // Notifies that a clipboard data transfer has completed (successfully or not).
    // Must be called after SelectionWrite with the same serial.
    // success: true if the write succeeded, false otherwise
    method SelectionWriteDone(
        session_handle: OwnedObjectPath,
        serial: u32,
        success: bool,
    ) -> ();

    // Transfer clipboard content for the given MIME type to the caller via an FD.
    // The callee (compositor) creates the file descriptor.
    // mime_type: the requested clipboard MIME type
    // fd (out): FD from which the caller can read the clipboard data
    method SelectionRead(
        session_handle: OwnedObjectPath,
        mime_type: String,
    ) -> (fd: OwnedFd);

    // Emitted when the clipboard selection ownership changes.
    // options keys:
    //   "mime_types":       as  — MIME types of the new selection
    //   "session_is_owner": b   — true if this session is now the owner
    signal SelectionOwnerChanged(
        session_handle: OwnedObjectPath,
        options: HashMap<String, OwnedValue>,
    );

    // Emitted to request the owning session to deliver clipboard data.
    // The session must call SelectionWrite(session_handle, serial) then
    // SelectionWriteDone(session_handle, serial, success).
    // mime_type: the MIME type requested
    // serial: identifies this specific transfer; used to pair with SelectionWrite
    signal SelectionTransfer(
        session_handle: OwnedObjectPath,
        mime_type: String,
        serial: u32,
    );
);

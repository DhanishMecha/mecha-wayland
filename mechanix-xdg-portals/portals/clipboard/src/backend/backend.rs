use std::collections::HashMap;

use app::{RegisteredModule, prelude::*};
use dbus::{DbusEvent, DbusMessage, DbusProxy, IncomingCall, SessionBus, fdo, variant};
use zbus::message::Message;
use zbus::zvariant::{OwnedValue, Value};

use super::interface::{
    CLIPBOARD_IFACE, CLIPBOARD_VERSION, RequestClipboard, SelectionOwnerChanged, SelectionRead,
    SelectionTransfer, SelectionWrite, SelectionWriteDone, SetSelection,
};
use super::types::{ClipboardSession, SessionHandle};
use portal_core::{PORTAL_PATH, RequestClose};
#[derive(State)]
pub struct ClipboardBackend {
    proxy: DbusProxy<SessionBus>,
    sessions: HashMap<SessionHandle, ClipboardSession>,
    /// Pending write-end FDs keyed by transfer serial.
    /// Populated in SelectionRead; consumed in SelectionWrite.
    pending_transfers: HashMap<u32, std::os::fd::OwnedFd>,
    /// Pending SelectionRead D-Bus calls from reader B waiting for SelectionWriteDone.
    /// Keyed by transfer serial: serial → (raw D-Bus message, read-end FD).
    pending_read_calls: HashMap<u32, (std::rc::Rc<Message>, zbus::zvariant::OwnedFd)>,
}

impl ClipboardBackend {
    pub fn new(proxy: DbusProxy<SessionBus>) -> Self {
        Self {
            proxy,
            sessions: HashMap::new(),
            pending_transfers: HashMap::new(),
            pending_read_calls: HashMap::new(),
        }
    }

    /// Find the session that currently owns the clipboard selection.
    fn owner_session(&self) -> Option<&ClipboardSession> {
        self.sessions.values().find(|s| s.is_owner)
    }

    /// Emit `SelectionOwnerChanged` to all registered sessions.
    fn broadcast_selection_owner_changed(&self, mime_types: &[String], owner_handle: &str) {
        let mt_values: Vec<Value<'_>> = mime_types.iter().map(|m| Value::Str(m.into())).collect();
        let mime_owned = OwnedValue::try_from(Value::Array(mt_values.into()))
            .unwrap_or_else(|_| OwnedValue::try_from(Value::Str("".into())).unwrap());

        for (handle, _) in &self.sessions {
            let is_owner = handle == owner_handle;
            let mut opts: HashMap<String, OwnedValue> = HashMap::new();
            opts.insert("mime_types".to_string(), mime_owned.clone());
            opts.insert(
                "session_is_owner".to_string(),
                OwnedValue::try_from(Value::Bool(is_owner)).unwrap(),
            );
            if let Ok(path) = zbus::zvariant::OwnedObjectPath::try_from(handle.clone()) {
                self.proxy
                    .emit::<SelectionOwnerChanged>(PORTAL_PATH, &(path, opts));
            }
        }
    }
}

pub fn clipboard_backend_module<S>() -> impl RegisteredModule<ClipboardBackend, S> {
    Module::<ClipboardBackend, _, _>::new().on(
        |s: &mut ClipboardBackend, ev: &DbusEvent<SessionBus>| -> Option<()> {
            // RequestClipboard
            // Attach clipboard access to an existing session before it starts.
            if let Some(Ok(call)) = IncomingCall::<RequestClipboard>::try_from(&ev.msg) {
                let (session_handle, _options) = &call.args;
                let handle_str = session_handle.as_str().to_string();
                println!(
                    "[clipboard-backend] RequestClipboard: session={}",
                    handle_str
                );

                s.sessions
                    .entry(handle_str.clone())
                    .or_insert_with(|| ClipboardSession::new(handle_str));

                call.respond(&s.proxy, &());
                return None;
            }

            // SetSelection
            // Session claims clipboard ownership and advertises its MIME types.
            // Emits SelectionOwnerChanged to all other sessions.
            // options keys:
            //   "mime_types": as  — MIME types this session provides
            if let Some(Ok(call)) = IncomingCall::<SetSelection>::try_from(&ev.msg) {
                let (session_handle, options) = &call.args;
                let handle_str = session_handle.as_str().to_string();

                let mime_types: Vec<String> = options
                    .get("mime_types")
                    .and_then(|v| Vec::<String>::try_from(v.clone()).ok())
                    .unwrap_or_default();

                println!(
                    "[clipboard-backend] SetSelection: session={} mime_types={:?}",
                    handle_str, mime_types
                );

                // Update session ownership
                for (h, session) in s.sessions.iter_mut() {
                    session.is_owner = h == &handle_str;
                    if h == &handle_str {
                        session.mime_types = mime_types.clone();
                    }
                }

                let mime_types_clone = mime_types.clone();
                let handle_clone = handle_str.clone();
                s.broadcast_selection_owner_changed(&mime_types_clone, &handle_clone);

                call.respond(&s.proxy, &());
                return None;
            }

            // SelectionRead
            if let Some(Ok(call)) = IncomingCall::<SelectionRead>::try_from(&ev.msg) {
                let (session_handle, mime_type) = &call.args;
                let handle_str = session_handle.as_str().to_string();

                println!(
                    "[clipboard-backend] SelectionRead: session={} mime_type={}",
                    handle_str, mime_type
                );

                match std::os::unix::net::UnixStream::pair() {
                    Ok((read_end, write_end)) => {
                        let read_fd: std::os::fd::OwnedFd = read_end.into();
                        let write_fd: std::os::fd::OwnedFd = write_end.into();
                        let caller_fd = zbus::zvariant::OwnedFd::from(read_fd);

                        if let Some(owner) = s.sessions.values_mut().find(|s| s.is_owner) {
                            let serial = owner.alloc_serial();
                            let owner_handle = owner.session_handle.clone();

                            // Stash write-end for owner A (SelectionWrite)
                            s.pending_transfers.insert(serial, write_fd);
                            // Stash reader B's pending D-Bus call until SelectionWriteDone
                            s.pending_read_calls.insert(serial, (call.raw().clone(), caller_fd));

                            if let Ok(owner_path) =
                                zbus::zvariant::OwnedObjectPath::try_from(owner_handle)
                            {
                                s.proxy.emit::<SelectionTransfer>(
                                    PORTAL_PATH,
                                    &(owner_path, mime_type.clone(), serial),
                                );
                                println!(
                                    "[clipboard-backend] SelectionTransfer emitted: \
                                     mime_type={mime_type} serial={serial} (holding SelectionRead call open)"
                                );
                            }
                        } else {
                            println!(
                                "[clipboard-backend] SelectionRead: no owner session — \
                                 replying error to caller"
                            );
                            s.proxy.reply_error(
                                call.raw(),
                                "org.freedesktop.portal.Error.Failed",
                                "No session currently owns the clipboard",
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "[clipboard-backend] SelectionRead: failed to create socket pair: {e}"
                        );
                        s.proxy.reply_error(
                            call.raw(),
                            "org.freedesktop.portal.Error.Failed",
                            &format!("Could not create clipboard data pipe: {e}"),
                        );
                    }
                }
                return None;
            }

            // SelectionWrite
             if let Some(Ok(call)) = IncomingCall::<SelectionWrite>::try_from(&ev.msg) {
                let (session_handle, serial) = &call.args;
                println!(
                    "[clipboard-backend] SelectionWrite: session={} serial={}",
                    session_handle.as_str(),
                    serial
                );

                match s.pending_transfers.remove(serial) {
                    Some(write_fd) => {
                        let owned_fd = zbus::zvariant::OwnedFd::from(write_fd);
                        println!(
                            "[clipboard-backend] SelectionWrite: handing write-end to owner \
                             session={} serial={serial}",
                            session_handle.as_str()
                        );
                        s.proxy.reply(call.raw(), &(owned_fd,));
                    }
                    None => {
                        eprintln!(
                            "[clipboard-backend] SelectionWrite: no pending transfer for \
                             serial={serial} (session={})",
                            session_handle.as_str()
                        );
                        s.proxy.reply_error(
                            call.raw(),
                            "org.freedesktop.portal.Error.Failed",
                            &format!("No pending SelectionRead for serial {serial}"),
                        );
                    }
                }
                return None;
            }

            // SelectionWriteDone
            if let Some(Ok(call)) = IncomingCall::<SelectionWriteDone>::try_from(&ev.msg) {
                let (session_handle, serial, success) = &call.args;
                println!(
                    "[clipboard-backend] SelectionWriteDone: session={} serial={} success={}",
                    session_handle.as_str(),
                    serial,
                    success
                );

                if let Some((raw_read_msg, read_fd)) = s.pending_read_calls.remove(serial) {
                    if *success {
                        println!(
                            "[clipboard-backend] SelectionWriteDone: success — \
                             replying read-end FD to reader B (serial={serial})"
                        );
                        s.proxy.reply(&raw_read_msg, &(read_fd,));
                    } else {
                        println!(
                            "[clipboard-backend] SelectionWriteDone: failed — \
                             replying D-Bus error to reader B (serial={serial})"
                        );
                        s.proxy.reply_error(
                            &raw_read_msg,
                            "org.freedesktop.portal.Error.Failed",
                            "Clipboard data transfer failed",
                        );
                    }
                } else {
                    println!(
                        "[clipboard-backend] SelectionWriteDone: no pending SelectionRead call for serial={serial}"
                    );
                }

                // Clean up any remaining write-end FD
                s.pending_transfers.remove(serial);

                call.respond(&s.proxy, &());
                return None;
            }

            // Request.Close
            // Removes the clipboard session when the underlying session is closed.
            if let Some(Ok(call)) = IncomingCall::<RequestClose>::try_from(&ev.msg) {
                if let Some(handle) = &call.path {
                    call.respond(&s.proxy, &());
                    if s.sessions.remove(handle).is_some() {
                        println!("[clipboard-backend] Session closed: handle={}", handle);
                    }
                }
                return None;
            }

            // Properties
            if fdo::route_properties(
                &s.proxy,
                &ev.msg,
                CLIPBOARD_IFACE,
                &["version"],
                |access| match access {
                    fdo::PropAccess::Get("version") => {
                        fdo::PropReply::Value(variant(CLIPBOARD_VERSION))
                    }
                    fdo::PropAccess::Set("version", _) => fdo::PropReply::ReadOnly,
                    _ => fdo::PropReply::Unknown,
                },
            ) {
                return None;
            }

            // Fallback
            if let DbusMessage::Call(m) = &ev.msg {
                if m.header()
                    .path()
                    .is_some_and(|p| p.as_str() == PORTAL_PATH)
                    && m.header()
                        .interface()
                        .is_some_and(|i| i.as_str() == CLIPBOARD_IFACE)
                {
                    eprintln!(
                        "[clipboard-backend] FALLBACK: unknown method={:?} on Clipboard interface",
                        m.header().member()
                    );
                    s.proxy.reply_unknown_method(m);
                }
            }

            None
        },
    )
}

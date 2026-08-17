use std::collections::HashMap;
use std::rc::Rc;

use app::{RegisteredModule, prelude::*};
use dbus::{DbusEvent, DbusMessage, DbusProxy, IncomingCall, SessionBus, fdo, variant};
use zbus::message::Message;

use super::interface::{
    ConnectToEIS, CreateSession, NotifyKeyboardKeycode, NotifyKeyboardKeysym, NotifyPointerAxis,
    NotifyPointerAxisDiscrete, NotifyPointerButton, NotifyPointerMotion,
    NotifyPointerMotionAbsolute, NotifyTouchDown, NotifyTouchMotion, NotifyTouchUp,
    REMOTE_DESKTOP_IFACE, REMOTE_DESKTOP_VERSION, SelectDevices, Start,
};
use super::types::{
    CreateSessionResults, DeviceType, PersistMode, RemoteDesktopOutcome, RemoteDesktopRequest,
    RemoteDesktopResponse, RemoteDesktopSession, RequestHandle, RequestKind, SelectDevicesResults,
    StartResults, StreamOptions,
};
use portal_core::{PORTAL_PATH, RESPONSE_CANCELLED, RESPONSE_SUCCESS, RequestClose};

#[derive(State)]
pub struct RemoteDesktopBackend {
    proxy: DbusProxy<SessionBus>,
    /// Requests awaiting user confirmation: handle → (raw message, kind).
    pending: HashMap<RequestHandle, (Rc<Message>, RequestKind)>,
    /// Active sessions: session_handle → session state.
    sessions: HashMap<String, RemoteDesktopSession>,
}

impl RemoteDesktopBackend {
    pub fn new(proxy: DbusProxy<SessionBus>) -> Self {
        Self {
            proxy,
            pending: HashMap::new(),
            sessions: HashMap::new(),
        }
    }

    /// Store a pending request so it can be resolved once the user responds.
    fn stash_pending(
        &mut self,
        handle: &zbus::zvariant::OwnedObjectPath,
        raw: &Rc<Message>,
        kind: RequestKind,
    ) -> RequestHandle {
        let handle_str = handle.as_str().to_string();
        self.pending
            .insert(handle_str.clone(), (Rc::clone(raw), kind));
        handle_str
    }

    /// Resolve a pending request with the given outcome and reply to D-Bus.
    fn finish(&mut self, handle: &str, outcome: RemoteDesktopOutcome) {
        let Some((raw, kind)) = self.pending.remove(handle) else {
            return;
        };
        match (outcome, kind) {
            (RemoteDesktopOutcome::Granted { granted_devices }, RequestKind::Start) => {
                // Note: The PipeWire stream node is not created or sent from here.
                // It is instead managed and returned by the ScreenCast portal backend.
                // RemoteDesktop only references the devices granted for virtual input.
                let results = StartResults {
                    devices: Some(granted_devices),
                    clipboard_enabled: Some(false),
                    streams: None,
                    restore_data: None,
                };
                self.proxy.reply(&raw, &(RESPONSE_SUCCESS, results));
            }
            (RemoteDesktopOutcome::Denied, RequestKind::Start) => {
                self.proxy
                    .reply(&raw, &(RESPONSE_CANCELLED, StartResults::default()));
            }
        }
    }
}

// Module constructor

pub fn remote_desktop_backend_module<S>() -> impl RegisteredModule<RemoteDesktopBackend, S> {
    Module::<RemoteDesktopBackend, _, _>::new()
        // Handle the UI response and resolve the pending D-Bus call.
        .on(|s: &mut RemoteDesktopBackend, done: &RemoteDesktopResponse| {
            s.finish(&done.handle, done.outcome.clone());
        })
        // Dispatch incoming D-Bus calls.
        .on(
            |s: &mut RemoteDesktopBackend,
             ev: &DbusEvent<SessionBus>|
             -> Option<RemoteDesktopRequest> {
                // CreateSession()
                // The spec says CreateSession acknowledges the session setup
                // immediately; no user interaction is required at this stage.
                if let Some(Ok(call)) = IncomingCall::<CreateSession>::try_from(&ev.msg) {
                    let (_handle, session_handle, app_id, options) = &call.args;
                    let session_handle_str = session_handle.as_str().to_string();

                    println!(
                        "[remote-desktop-backend] CreateSession requested by app={app_id} \
                         session={session_handle_str}"
                    );

                    // Reject duplicate session handles – each session object path
                    // must be unique per the portal spec.
                    if s.sessions.contains_key(&session_handle_str) {
                        eprintln!(
                            "[remote-desktop-backend] CreateSession rejected: \
                             session {session_handle_str} already exists"
                        );
                        s.proxy.reply_error(
                            call.raw(),
                            "org.freedesktop.portal.Error.InvalidArgument",
                            "A session with this handle already exists",
                        );
                        return None;
                    }

                    // Pre-register a nascent session so SelectDevices can verify
                    // the session was legitimately created before configuring it.
                    s.sessions.insert(
                        session_handle_str,
                        RemoteDesktopSession {
                            device_types: DeviceType::ALL,
                            persist_mode: PersistMode::NoPersist,
                            devices_selected: false,
                            closed: false,
                        },
                    );

                    s.proxy.reply(call.raw(), &(RESPONSE_SUCCESS, CreateSessionResults {}));
                    return None;
                }

                // SelectDevices()
                if let Some(Ok(call)) = IncomingCall::<SelectDevices>::try_from(&ev.msg) {
                    let (_handle, session_handle, app_id, options) = &call.args;
                    let session_handle_str = session_handle.as_str().to_string();

                    // Verify the session was created by a prior CreateSession call.
                    let Some(session) = s.sessions.get_mut(&session_handle_str) else {
                        eprintln!(
                            "[remote-desktop-backend] SelectDevices rejected: \
                             unknown session {session_handle_str}"
                        );
                        s.proxy.reply(
                            call.raw(),
                            &(RESPONSE_CANCELLED, SelectDevicesResults {}),
                        );
                        return None;
                    };

                    // Reject if the session has already been closed.
                    if session.closed {
                        eprintln!(
                            "[remote-desktop-backend] SelectDevices rejected: \
                             session {session_handle_str} is already closed"
                        );
                        s.proxy.reply(
                            call.raw(),
                            &(RESPONSE_CANCELLED, SelectDevicesResults {}),
                        );
                        return None;
                    }

                    // Decode and validate the requested device type bitmask.
                    // The spec says the default is all available types when
                    // `types` is absent; an explicit 0 or a value with no overlap
                    // against AvailableDeviceTypes is an error.
                    let requested_types = options.types.unwrap_or(DeviceType::ALL);
                    if requested_types == 0 || (requested_types & DeviceType::ALL == 0) {
                        eprintln!(
                            "[remote-desktop-backend] SelectDevices rejected: \
                             invalid device types bitmask 0x{requested_types:x} \
                             (available: 0x{:x})",
                            DeviceType::ALL
                        );
                        s.proxy.reply(
                            call.raw(),
                            &(RESPONSE_CANCELLED, SelectDevicesResults {}),
                        );
                        return None;
                    }
                    // Mask to only the bits we actually support.
                    let types = requested_types & DeviceType::ALL;

                    let persist_mode = match options.persist_mode.unwrap_or(0) {
                        1 => PersistMode::WhileRunning,
                        2 => PersistMode::UntilRevoked,
                        _ => PersistMode::NoPersist,
                    };

                    println!(
                        "[remote-desktop-backend] SelectDevices: app={app_id} \
                         session={session_handle_str} types=0x{types:x} \
                         persist_mode={persist_mode:?}"
                    );

                    // Handle optional restore_data (v2): if present and the
                    // session data can be restored, we would skip the prompt;
                    // for now we log and ignore it (full restore is a TODO).
                    if let Some(restore_data) = &options.restore_data {
                        let (vendor, version, _data) = restore_data;
                        println!(
                            "[remote-desktop-backend] SelectDevices: restore_data \
                             present vendor={vendor} version={version} – \
                             session restore not yet implemented, ignoring"
                        );
                        // TODO: attempt to restore persisted session; if successful
                        // skip the Start dialog and reply immediately with the
                        // previously granted device set.
                    }

                    // Commit the updated configuration into the session record.
                    session.device_types = types;
                    session.persist_mode = persist_mode;
                    session.devices_selected = true;

                    s.proxy.reply(call.raw(), &(RESPONSE_SUCCESS, SelectDevicesResults {}));
                    return None;
                }

                // Start()
                if let Some(Ok(call)) = IncomingCall::<Start>::try_from(&ev.msg) {
                    let (handle, session_handle, app_id, _parent_window, _options) = &call.args;
                    let session_handle_str = session_handle.as_str().to_string();

                    // Verify the session exists and was configured via SelectDevices.
                    let (device_types, closed, devices_selected) = match s.sessions.get(&session_handle_str) {
                        Some(sess) => (sess.device_types, sess.closed, sess.devices_selected),
                        None => {
                            eprintln!(
                                "[remote-desktop-backend] Start rejected: \
                                 unknown session {session_handle_str}"
                            );
                            s.proxy.reply(call.raw(), &(RESPONSE_CANCELLED, StartResults::default()));
                            return None;
                        }
                    };

                    if closed {
                        eprintln!(
                            "[remote-desktop-backend] Start rejected: \
                             session {session_handle_str} is already closed"
                        );
                        s.proxy.reply(call.raw(), &(RESPONSE_CANCELLED, StartResults::default()));
                        return None;
                    }

                    if !devices_selected {
                        eprintln!(
                            "[remote-desktop-backend] Start rejected: \
                             SelectDevices has not been called for session {session_handle_str}"
                        );
                        s.proxy.reply(call.raw(), &(RESPONSE_CANCELLED, StartResults::default()));
                        return None;
                    }

                    println!(
                        "[remote-desktop-backend] Start: app={app_id} \
                         session={session_handle_str} device_types=0x{device_types:x}"
                    );

                    return Some(RemoteDesktopRequest {
                        handle: s.stash_pending(handle, call.raw(), RequestKind::Start),
                        session_handle: session_handle_str,
                        app_id: app_id.clone(),
                        device_types,
                    });
                }

                // NotifyPointerMotion()
                if let Some(Ok(call)) = IncomingCall::<NotifyPointerMotion>::try_from(&ev.msg) {
                    let (session_handle, _options, dx, dy) = &call.args;
                    println!(
                        "[remote-desktop-backend] NotifyPointerMotion dx={dx} dy={dy}"
                    );
                    // TODO: forward (dx, dy) relative motion to compositor / EIS
                    s.proxy.reply(call.raw(), &());
                    return None;
                }

                // NotifyPointerMotionAbsolute()
                if let Some(Ok(call)) =
                    IncomingCall::<NotifyPointerMotionAbsolute>::try_from(&ev.msg)
                {
                    let (session_handle, _options, stream, x, y) = &call.args;
                    println!(
                        "[remote-desktop-backend] NotifyPointerMotionAbsolute \
                         stream={stream} x={x} y={y}"
                    );
                    // TODO: forward absolute pointer position to compositor / EIS
                    s.proxy.reply(call.raw(), &());
                    return None;
                }

                // NotifyPointerButton()
                if let Some(Ok(call)) = IncomingCall::<NotifyPointerButton>::try_from(&ev.msg) {
                    let (session_handle, _options, button, state) = &call.args;
                    println!(
                        "[remote-desktop-backend] NotifyPointerButton \
                         button={button} state={state}"
                    );
                    // TODO: forward button event to compositor / EIS
                    s.proxy.reply(call.raw(), &());
                    return None;
                }

                // NotifyPointerAxis()
                if let Some(Ok(call)) = IncomingCall::<NotifyPointerAxis>::try_from(&ev.msg) {
                    let (session_handle, options, dx, dy) = &call.args;
                    let finish = options.finish.unwrap_or(false);
                    println!(
                        "[remote-desktop-backend] NotifyPointerAxis \
                         dx={dx} dy={dy} finish={finish}"
                    );
                    // TODO: forward smooth-scroll axis event to compositor / EIS
                    s.proxy.reply(call.raw(), &());
                    return None;
                }

                // NotifyPointerAxisDiscrete()
                if let Some(Ok(call)) =
                    IncomingCall::<NotifyPointerAxisDiscrete>::try_from(&ev.msg)
                {
                    let (session_handle, _options, axis, steps) = &call.args;
                    println!(
                        "[remote-desktop-backend] NotifyPointerAxisDiscrete \
                         axis={axis} steps={steps}"
                    );
                    // TODO: forward discrete scroll event to compositor / EIS
                    s.proxy.reply(call.raw(), &());
                    return None;
                }

                // NotifyKeyboardKeycode()
                if let Some(Ok(call)) =
                    IncomingCall::<NotifyKeyboardKeycode>::try_from(&ev.msg)
                {
                    let (session_handle, _options, keycode, state) = &call.args;
                    println!(
                        "[remote-desktop-backend] NotifyKeyboardKeycode \
                         keycode={keycode} state={state}"
                    );
                    // TODO: forward keycode event to compositor / EIS
                    s.proxy.reply(call.raw(), &());
                    return None;
                }

                // NotifyKeyboardKeysym()
                if let Some(Ok(call)) =
                    IncomingCall::<NotifyKeyboardKeysym>::try_from(&ev.msg)
                {
                    let (session_handle, _options, keysym, state) = &call.args;
                    println!(
                        "[remote-desktop-backend] NotifyKeyboardKeysym \
                         keysym={keysym} state={state}"
                    );
                    // TODO: forward keysym event to compositor / EIS
                    s.proxy.reply(call.raw(), &());
                    return None;
                }

                // NotifyTouchDown()
                if let Some(Ok(call)) = IncomingCall::<NotifyTouchDown>::try_from(&ev.msg) {
                    let (session_handle, _options, stream, slot, x, y) = &call.args;
                    println!(
                        "[remote-desktop-backend] NotifyTouchDown \
                         stream={stream} slot={slot} x={x} y={y}"
                    );
                    // TODO: forward touch-down event to compositor / EIS
                    s.proxy.reply(call.raw(), &());
                    return None;
                }

                // NotifyTouchMotion()
                if let Some(Ok(call)) = IncomingCall::<NotifyTouchMotion>::try_from(&ev.msg) {
                    let (session_handle, _options, stream, slot, x, y) = &call.args;
                    println!(
                        "[remote-desktop-backend] NotifyTouchMotion \
                         stream={stream} slot={slot} x={x} y={y}"
                    );
                    // TODO: forward touch-motion event to compositor / EIS
                    s.proxy.reply(call.raw(), &());
                    return None;
                }

                // NotifyTouchUp()
                if let Some(Ok(call)) = IncomingCall::<NotifyTouchUp>::try_from(&ev.msg) {
                    let (session_handle, _options, slot) = &call.args;
                    println!(
                        "[remote-desktop-backend] NotifyTouchUp slot={slot}"
                    );
                    // TODO: forward touch-up event to compositor / EIS
                    s.proxy.reply(call.raw(), &());
                    return None;
                }

                // ConnectToEIS()  [v2]
                if let Some(Ok(call)) = IncomingCall::<ConnectToEIS>::try_from(&ev.msg) {
                    let (session_handle, app_id, _options) = &call.args;
                    println!(
                        "[remote-desktop-backend] ConnectToEIS for session={} app={app_id}",
                        session_handle.as_str()
                    );
                    // TODO: create and return a real EIS socket fd
                    // For now, return an error until EIS support is implemented.
                    s.proxy.reply_error(
                        call.raw(),
                        "org.freedesktop.portal.Error.NotSupported",
                        "EIS connection is not yet implemented",
                    );
                    return None;
                }

                // Request.Close()
                if let Some(Ok(call)) = IncomingCall::<RequestClose>::try_from(&ev.msg) {
                    if let Some(handle) = &call.path {
                        call.respond(&s.proxy, &());
                        if s.pending.contains_key(handle) {
                            let h = handle.clone();
                            s.finish(&h, RemoteDesktopOutcome::Denied);
                        }
                    }
                    return None;
                }

                // Properties
                if fdo::route_properties(
                    &s.proxy,
                    &ev.msg,
                    REMOTE_DESKTOP_IFACE,
                    &["version", "AvailableDeviceTypes"],
                    |access| match access {
                        fdo::PropAccess::Get("version") => {
                            fdo::PropReply::Value(variant(REMOTE_DESKTOP_VERSION))
                        }
                        fdo::PropAccess::Get("AvailableDeviceTypes") => {
                            fdo::PropReply::Value(variant(DeviceType::ALL))
                        }
                        fdo::PropAccess::Set("version", _)
                        | fdo::PropAccess::Set("AvailableDeviceTypes", _) => {
                            fdo::PropReply::ReadOnly
                        }
                        _ => fdo::PropReply::Unknown,
                    },
                ) {
                    return None;
                }

                // Fallback: unknown method on our interface
                if let DbusMessage::Call(m) = &ev.msg {
                    if m.header().path().is_some_and(|p| p.as_str() == PORTAL_PATH)
                        && m.header()
                            .interface()
                            .is_some_and(|i| i.as_str() == REMOTE_DESKTOP_IFACE)
                    {
                        eprintln!(
                            "[remote-desktop-backend] FALLBACK: unknown method on RemoteDesktop interface"
                        );
                        s.proxy.reply_unknown_method(m);
                    }
                }

                None
            },
        )
}

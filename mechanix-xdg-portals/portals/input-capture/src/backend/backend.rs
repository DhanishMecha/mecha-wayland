use std::collections::HashMap;
use std::rc::Rc;

use app::{RegisteredModule, prelude::*};
use dbus::{DbusEvent, DbusMessage, DbusProxy, IncomingCall, SessionBus, fdo, variant};
use zbus::message::Message;
use zbus::zvariant::{OwnedFd, OwnedValue, Value};

use super::interface::{
    Activated, ConnectToEIS, CreateSession, CreateSession2, Deactivated, Disable, Enable, GetZones,
    INPUTCAPTURE_IFACE, INPUTCAPTURE_VERSION, Release, SetPointerBarriers, Start,
};
use super::types::{
    InputCaptureOutcome, InputCaptureRequest, InputCaptureResponse, InputCaptureSession,
    RequestHandle, RequestKind,
};
use portal_core::{PORTAL_PATH, RESPONSE_CANCELLED, RESPONSE_SUCCESS, RequestClose};

#[derive(State)]
pub struct InputCaptureBackend {
    proxy: DbusProxy<SessionBus>,
    pending: HashMap<RequestHandle, (Rc<Message>, RequestKind)>,
    sessions: HashMap<String, InputCaptureSession>,
}

fn get_u32_prop(map: &HashMap<String, OwnedValue>, key: &str) -> Option<u32> {
    map.get(key).and_then(|v| {
        if let Ok(u) = u32::try_from(v.clone()) {
            Some(u)
        } else {
            let inner = Value::from(v.clone());
            if let Value::Value(boxed) = inner {
                u32::try_from(*boxed).ok()
            } else {
                None
            }
        }
    })
}

fn open_eis_fd() -> std::io::Result<OwnedFd> {
    // TODO: Connect to the actual EIS socket/instance provided by the compositor instead of returning a stub socket pair
    let (s1, _s2) = std::os::unix::net::UnixStream::pair()?;
    let std_fd: std::os::fd::OwnedFd = s1.into();
    Ok(OwnedFd::from(std_fd))
}

impl InputCaptureBackend {
    pub fn new(proxy: DbusProxy<SessionBus>) -> Self {
        Self {
            proxy,
            pending: HashMap::new(),
            sessions: HashMap::new(),
        }
    }

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

    fn finish(&mut self, handle: &str, outcome: InputCaptureOutcome) {
        let Some((raw, kind)) = self.pending.remove(handle) else {
            return;
        };

        match (outcome, kind) {
            (InputCaptureOutcome::Granted { capabilities }, RequestKind::Start) => {
                println!(
                    "[input-capture-backend] Granted capabilities={capabilities} for handle={handle}"
                );
                let mut results = HashMap::new();
                results.insert(
                    "capabilities".to_string(),
                    OwnedValue::try_from(Value::from(capabilities)).unwrap(),
                );
                // Return success (0) and results dictionary
                self.proxy.reply(&raw, &(RESPONSE_SUCCESS, results));
            }
            (InputCaptureOutcome::Denied, RequestKind::Start) => {
                println!("[input-capture-backend] Denied for handle={handle}");
                self.proxy.reply(
                    &raw,
                    &(RESPONSE_CANCELLED, HashMap::<String, OwnedValue>::new()),
                );
            }
        }
    }
}

pub fn input_capture_backend_module<S>() -> impl RegisteredModule<InputCaptureBackend, S> {
    Module::<InputCaptureBackend, _, _>::new()
        .on(|s: &mut InputCaptureBackend, done: &InputCaptureResponse| {
            s.finish(&done.handle, done.outcome.clone());
        })
        .on(
            |s: &mut InputCaptureBackend, ev: &DbusEvent<SessionBus>| -> Option<InputCaptureRequest> {
                // CreateSession (deprecated)
                if let Some(Ok(call)) = IncomingCall::<CreateSession>::try_from(&ev.msg) {
                    let (_handle, session_handle, app_id, _parent, options) = &call.args;
                    let session_handle_str = session_handle.as_str().to_string();
                    println!(
                        "[input-capture-backend] CreateSession app_id={app_id} session={session_handle_str}"
                    );

                    let caps = get_u32_prop(options, "capabilities").unwrap_or(0);
                    let session = InputCaptureSession {
                        capabilities: caps,
                        app_id: app_id.clone(),
                        closed: false,
                    };
                    s.sessions.insert(session_handle_str, session);

                    call.respond(&s.proxy, &(0u32, HashMap::<String, OwnedValue>::new()));
                    return None;
                }

                // CreateSession2 - new method
                if let Some(Ok(call)) = IncomingCall::<CreateSession2>::try_from(&ev.msg) {
                    let (session_handle, app_id, _options) = &call.args;
                    let session_handle_str = session_handle.as_str().to_string();
                    println!(
                        "[input-capture-backend] CreateSession2 app_id={app_id} session={session_handle_str}"
                    );

                    // TODO: Handle restore_data option for session persistence

                    let session = InputCaptureSession {
                        capabilities: 0, // Not started/configured yet
                        app_id: app_id.clone(),
                        closed: false,
                    };
                    s.sessions.insert(session_handle_str, session);

                    call.respond(&s.proxy, &(HashMap::<String, OwnedValue>::new(),));
                    return None;
                }

                // Start
                if let Some(Ok(call)) = IncomingCall::<Start>::try_from(&ev.msg) {
                    let (handle, session_handle, app_id, parent_window, options) = &call.args;
                    let session_handle_str = session_handle.as_str().to_string();
                    let caps = get_u32_prop(options, "capabilities").unwrap_or(0);
                    println!(
                        "[input-capture-backend] Start app_id={app_id} parent={parent_window} \
                         session={session_handle_str} caps={caps}"
                    );

                    // TODO: Return restore_data in the response results if persist_mode is enabled

                    if let Some(sess) = s.sessions.get_mut(&session_handle_str) {
                        sess.capabilities = caps;
                    }

                    let handle_str = s.stash_pending(handle, call.raw(), RequestKind::Start);
                    return Some(InputCaptureRequest::Start {
                        handle: handle_str,
                        session_handle: session_handle_str,
                        app_id: app_id.clone(),
                        capabilities: caps,
                    });
                }

                // GetZones
                if let Some(Ok(call)) = IncomingCall::<GetZones>::try_from(&ev.msg) {
                    let (_handle, session_handle, app_id, _options) = &call.args;
                    let session_handle_str = session_handle.as_str().to_string();
                    println!(
                        "[input-capture-backend] GetZones app_id={app_id} session={session_handle_str}"
                    );

                    // TODO: Retrieve actual zones dynamically from the compositor / display server instead of returning hardcoded values
                    let mut results = HashMap::new();
                    let zones: Vec<(u32, u32, i32, i32)> = vec![(1920, 1080, 0, 0)];
                    results.insert(
                        "zones".to_string(),
                        OwnedValue::try_from(Value::from(zones)).unwrap(),
                    );
                    results.insert(
                        "zone_set".to_string(),
                        OwnedValue::try_from(Value::from(1u32)).unwrap(),
                    );

                    call.respond(&s.proxy, &(0u32, results));
                    return None;
                }

                // SetPointerBarriers
                if let Some(Ok(call)) = IncomingCall::<SetPointerBarriers>::try_from(&ev.msg) {
                    let (_handle, session_handle, app_id, _options, barriers, zone_set) = &call.args;
                    let session_handle_str = session_handle.as_str().to_string();
                    println!(
                        "[input-capture-backend] SetPointerBarriers app_id={app_id} \
                         session={session_handle_str} barriers={} zone_set={zone_set}",
                        barriers.len()
                    );

                    // TODO: Implement actual pointer barrier setting with the Wayland compositor and report failed barriers if any
                    let mut results = HashMap::new();
                    let failed: Vec<u32> = Vec::new();
                    results.insert(
                        "failed_barriers".to_string(),
                        OwnedValue::try_from(Value::from(failed)).unwrap(),
                    );

                    call.respond(&s.proxy, &(0u32, results));
                    return None;
                }

                // Enable
                if let Some(Ok(call)) = IncomingCall::<Enable>::try_from(&ev.msg) {
                    let (session_handle, app_id, _options) = &call.args;
                    let session_handle_str = session_handle.as_str().to_string();
                    println!(
                        "[input-capture-backend] Enable app_id={app_id} session={session_handle_str}"
                    );

                    // TODO: Coordinate with the Wayland compositor to enable input capture and dynamically emit Activated/Deactivated/Disabled signals
                    call.respond(&s.proxy, &(0u32, HashMap::<String, OwnedValue>::new()));

                    // Proactively emit Activated signal
                    let mut opts = HashMap::new();
                    opts.insert(
                        "activation_id".to_string(),
                        OwnedValue::try_from(Value::from(1u32)).unwrap(),
                    );
                    s.proxy.emit::<Activated>(PORTAL_PATH, &(session_handle.clone(), opts));
                    return None;
                }

                // Disable
                if let Some(Ok(call)) = IncomingCall::<Disable>::try_from(&ev.msg) {
                    let (session_handle, app_id, _options) = &call.args;
                    let session_handle_str = session_handle.as_str().to_string();
                    println!(
                        "[input-capture-backend] Disable app_id={app_id} session={session_handle_str}"
                    );

                    // TODO: Coordinate with the Wayland compositor to disable input capture
                    call.respond(&s.proxy, &(0u32, HashMap::<String, OwnedValue>::new()));

                    // Proactively emit Deactivated signal
                    let mut opts = HashMap::new();
                    opts.insert(
                        "activation_id".to_string(),
                        OwnedValue::try_from(Value::from(1u32)).unwrap(),
                    );
                    s.proxy.emit::<Deactivated>(PORTAL_PATH, &(session_handle.clone(), opts));
                    return None;
                }

                // Release
                if let Some(Ok(call)) = IncomingCall::<Release>::try_from(&ev.msg) {
                    let (session_handle, app_id, _options) = &call.args;
                    let session_handle_str = session_handle.as_str().to_string();
                    println!(
                        "[input-capture-backend] Release app_id={app_id} session={session_handle_str}"
                    );

                    // TODO: Coordinate with the Wayland compositor to release the input capture session
                    call.respond(&s.proxy, &(0u32, HashMap::<String, OwnedValue>::new()));

                    // Proactively emit Deactivated signal
                    let mut opts = HashMap::new();
                    opts.insert(
                        "activation_id".to_string(),
                        OwnedValue::try_from(Value::from(1u32)).unwrap(),
                    );
                    s.proxy.emit::<Deactivated>(PORTAL_PATH, &(session_handle.clone(), opts));
                    return None;
                }

                // ConnectToEIS
                if let Some(Ok(call)) = IncomingCall::<ConnectToEIS>::try_from(&ev.msg) {
                    let (session_handle, app_id, _options) = &call.args;
                    let session_handle_str = session_handle.as_str().to_string();
                    println!(
                        "[input-capture-backend] ConnectToEIS app_id={app_id} session={session_handle_str}"
                    );

                    match open_eis_fd() {
                        Ok(fd) => {
                            call.respond(&s.proxy, &(fd,));
                        }
                        Err(e) => {
                            eprintln!("[input-capture-backend] ConnectToEIS failed: {e}");
                            s.proxy.reply_error(
                                call.raw(),
                                "org.freedesktop.portal.Error.Failed",
                                "Failed to open EIS connection",
                            );
                        }
                    }
                    return None;
                }

                // RequestClose
                if let Some(Ok(call)) = IncomingCall::<RequestClose>::try_from(&ev.msg) {
                    if let Some(handle) = &call.path {
                        call.respond(&s.proxy, &());
                        if s.pending.contains_key(handle) {
                            let h = handle.clone();
                            s.finish(&h, InputCaptureOutcome::Denied);
                            return Some(InputCaptureRequest::Close { handle: h });
                        }
                    }
                    return None;
                }

                // Properties
                if fdo::route_properties(
                    &s.proxy,
                    &ev.msg,
                    INPUTCAPTURE_IFACE,
                    &["version", "SupportedCapabilities"],
                    |access| match access {
                        fdo::PropAccess::Get("version") => {
                            fdo::PropReply::Value(variant(INPUTCAPTURE_VERSION))
                        }
                        fdo::PropAccess::Get("SupportedCapabilities") => {
                            fdo::PropReply::Value(variant(
                                super::types::InputCaptureCapability::ALL,
                            ))
                        }
                        fdo::PropAccess::Set("version", _)
                        | fdo::PropAccess::Set("SupportedCapabilities", _) => {
                            fdo::PropReply::ReadOnly
                        }
                        _ => fdo::PropReply::Unknown,
                    },
                ) {
                    return None;
                }

                // Fallback: unknown methods
                if let DbusMessage::Call(m) = &ev.msg {
                    if m.header().path().is_some_and(|p| p.as_str() == PORTAL_PATH)
                        && m.header()
                            .interface()
                            .is_some_and(|i| i.as_str() == INPUTCAPTURE_IFACE)
                    {
                        eprintln!(
                            "[input-capture-backend] FALLBACK: unknown method on InputCapture interface"
                        );
                        s.proxy.reply_unknown_method(m);
                    }
                }

                None
            },
        )
}

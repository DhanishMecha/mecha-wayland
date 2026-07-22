use std::collections::HashMap;
use std::rc::Rc;

use app::{prelude::*, RegisteredModule};
use dbus::{fdo, variant, DbusEvent, DbusMessage, DbusProxy, IncomingCall, SessionBus};
use zbus::message::Message;

use super::interface::{
    CreateSession, OpenPipeWireRemote, SelectSources, Start, SCREENCAST_IFACE, SCREENCAST_VERSION,
};
use super::types::{
    CreateSessionResults, CursorMode, RequestHandle, RequestKind, ScreenCastOutcome,
    ScreenCastRequest, ScreenCastResponse, SelectSourcesResults, SourceType, StartResults,
    StreamOptions, ScreencastSession,
};
use portal_core::{RequestClose, PORTAL_PATH, RESPONSE_CANCELLED, RESPONSE_SUCCESS};

#[derive(State)]
pub struct ScreenCastBackend {
    proxy: DbusProxy<SessionBus>,
    pending: HashMap<RequestHandle, (Rc<Message>, RequestKind)>,
    sessions: HashMap<String, ScreencastSession>,
}

impl ScreenCastBackend {
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

    fn finish(&mut self, handle: &str, outcome: ScreenCastOutcome) {
        let Some((raw, kind)) = self.pending.remove(handle) else {
            return;
        };
        match (outcome, kind) {
            (ScreenCastOutcome::Granted { selected_type }, RequestKind::Start) => {
                let stream_opt = StreamOptions {
                    // TODO: Get actual screen/window size dynamically instead of hardcoded 1080p
                    size: Some((1920, 1080)),
                    position: None,
                    source_type: Some(selected_type),
                    mapping_id: None,
                    // TODO: Get actual PipeWire stream serial dynamically
                    pipewire_serial: Some(1u64),
                    restore_data: None,
                };
                // PipeWire stream node ID 1
                let results = StartResults {
                    // TODO: Get actual PipeWire stream node ID dynamically instead of hardcoded 1
                    streams: Some(vec![(1u32, stream_opt)]),
                    persist_mode: None,
                    restore_data: None,
                };
                self.proxy.reply(&raw, &(RESPONSE_SUCCESS, results));
            }
            (ScreenCastOutcome::Denied, RequestKind::Start) => {
                self.proxy
                    .reply(&raw, &(RESPONSE_CANCELLED, StartResults::default()));
            }
        }
    }
}

fn open_pipewire_fd() -> std::io::Result<zbus::zvariant::OwnedFd> {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        let pw_path = std::path::Path::new(&runtime_dir).join("pipewire-0");
        print!("[screencast-backend] Looking for PipeWire socket at {}", pw_path.display());
        if pw_path.exists() {
            if let Ok(stream) = std::os::unix::net::UnixStream::connect(&pw_path) {
                let std_fd: std::os::fd::OwnedFd = stream.into();
                println!("[screencast-backend] Connected to PipeWire socket at {}", pw_path.display());
                return Ok(zbus::zvariant::OwnedFd::from(std_fd));
            }
        }
    }
    // Fallback socket pair if PipeWire is not running directly
    // TODO: Implement actual fallback or proper error handling if PipeWire is not running
    let (s1, _s2) = std::os::unix::net::UnixStream::pair()?;
    let std_fd: std::os::fd::OwnedFd = s1.into();
    Ok(zbus::zvariant::OwnedFd::from(std_fd))
}

pub fn screencast_backend_module<S>() -> impl RegisteredModule<ScreenCastBackend, S> {
    Module::<ScreenCastBackend, _, _>::new()
        .on(|s: &mut ScreenCastBackend, done: &ScreenCastResponse| {
            s.finish(&done.handle, done.outcome.clone());
        })
        .on(
            |s: &mut ScreenCastBackend, ev: &DbusEvent<SessionBus>| -> Option<ScreenCastRequest> {
                //  CreateSession()
                //  Per portal spec, CreateSession returns (0, {}) to acknowledge session setup.
                //  Actual source choices and stream details are returned in SelectSources() and Start().
                if let Some(Ok(call)) = IncomingCall::<CreateSession>::try_from(&ev.msg) {
                    let (_handle, _session_handle, app_id, _options) = &call.args;
                    println!("[screencast-backend] CreateSession requested by app={app_id}");
                    s.proxy
                        .reply(call.raw(), &(RESPONSE_SUCCESS, CreateSessionResults {}));
                    return None;
                }

                // SelectSources()
                if let Some(Ok(call)) = IncomingCall::<SelectSources>::try_from(&ev.msg) {
                    let (_handle, session_handle, app_id, options) = &call.args;
                    let types = options.types.unwrap_or(SourceType::Monitor.bits());
                    let multiple = options.multiple.unwrap_or(false);
                    let cursor_mode = options.cursor_mode.unwrap_or(CursorMode::Hidden.bits());
                    println!(
                        "[screencast-backend] SelectSources requested by app={app_id} \
                         types={types} multiple={multiple} cursor_mode={cursor_mode}"
                    );

                    // Validate parameters - if types are invalid/false, return RESPONSE_CANCELLED
                    if types == 0 || (types & SourceType::ALL == 0) {
                        eprintln!("[screencast-backend] Invalid source types requested: {types}");
                        s.proxy.reply(
                            call.raw(),
                            &(
                                RESPONSE_CANCELLED,
                                SelectSourcesResults { restore_data: None },
                            ),
                        );
                        return None;
                    }

                    let session_handle_str = session_handle.as_str().to_string();
                    let session = ScreencastSession {
                        cursor_mode: CursorMode::from_bits(cursor_mode),
                        multiple,
                        source_types: types,
                        persisted_capture_sources: None,
                        closed: false,
                    };
                    s.sessions.insert(session_handle_str, session);

                    s.proxy.reply(
                        call.raw(),
                        &(
                            RESPONSE_SUCCESS,
                            SelectSourcesResults { restore_data: None },
                        ),
                    );
                    return None;
                }

                //  Start()
                if let Some(Ok(call)) = IncomingCall::<Start>::try_from(&ev.msg) {
                    let (handle, session_handle, app_id, _parent, _options) = &call.args;
                    println!("[screencast-backend] Start requested by app={app_id}");
                    let session_handle_str = session_handle.as_str().to_string();
                    
                    // Get Session Details
                    let (types, cursor_mode, multiple) = s.sessions.get(&session_handle_str)
                        .map(|sess| (sess.source_types, sess.cursor_mode, sess.multiple))
                        // TODO: Handle missing session error instead of falling back to default monitor
                        .unwrap_or((SourceType::Monitor.bits(), None, false));

                    return Some(ScreenCastRequest::Start {
                        handle: s.stash_pending(handle, call.raw(), RequestKind::Start),
                        session_handle: session_handle_str,
                        app_id: app_id.clone(),
                        types,
                        cursor_mode,
                        multiple,
                    });
                }

                //  OpenPipeWireRemote()
                if let Some(Ok(call)) = IncomingCall::<OpenPipeWireRemote>::try_from(&ev.msg) {
                    let (session_handle, _options) = &call.args;
                    println!(
                        "[screencast-backend] OpenPipeWireRemote for session={}",
                        session_handle.as_str()
                    );
                    match open_pipewire_fd() {
                        Ok(fd) => {
                            s.proxy.reply(call.raw(), &(fd,));
                        }
                        Err(e) => {
                            eprintln!(
                                "[screencast-backend] Failed to open PipeWire remote fd: {e}"
                            );
                            s.proxy.reply_error(
                                call.raw(),
                                "org.freedesktop.portal.Error.Failed",
                                "Could not open PipeWire remote connection",
                            );
                        }
                    }
                    return None;
                }

                //  Request.Close()
                if let Some(Ok(call)) = IncomingCall::<RequestClose>::try_from(&ev.msg) {
                    if let Some(handle) = &call.path {
                        if s.pending.contains_key(handle) {
                            call.respond(&s.proxy, &());
                            let h = handle.clone();
                            s.finish(&h, ScreenCastOutcome::Denied);
                            return Some(ScreenCastRequest::Close { handle: h });
                        }
                    }
                    return None;
                }

                //  Properties
                if fdo::route_properties(
                    &s.proxy,
                    &ev.msg,
                    SCREENCAST_IFACE,
                    &["version", "AvailableSourceTypes", "AvailableCursorModes"],
                    |access| match access {
                        fdo::PropAccess::Get("version") => {
                            fdo::PropReply::Value(variant(SCREENCAST_VERSION))
                        }
                        fdo::PropAccess::Get("AvailableSourceTypes") => {
                            fdo::PropReply::Value(variant(SourceType::ALL))
                        }
                        fdo::PropAccess::Get("AvailableCursorModes") => {
                            fdo::PropReply::Value(variant(CursorMode::ALL))
                        }
                        fdo::PropAccess::Set("version", _)
                        | fdo::PropAccess::Set("AvailableSourceTypes", _)
                        | fdo::PropAccess::Set("AvailableCursorModes", _) => {
                            fdo::PropReply::ReadOnly
                        }
                        _ => fdo::PropReply::Unknown,
                    },
                ) {
                    return None;
                }

                //  Fallback
                if let DbusMessage::Call(m) = &ev.msg {
                    if m.header().path().is_some_and(|p| p.as_str() == PORTAL_PATH)
                        && m.header()
                            .interface()
                            .is_some_and(|i| i.as_str() == SCREENCAST_IFACE)
                    {
                        eprintln!(
                            "[screencast-backend] FALLBACK: unknown_method on ScreenCast interface"
                        );
                        s.proxy.reply_unknown_method(m);
                    }
                }

                None
            },
        )
}

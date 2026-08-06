use std::collections::HashMap;
use std::rc::Rc;

use app::{RegisteredModule, prelude::*};
use dbus::{
    DbusEvent, DbusMessage, DbusProxy, IncomingCall, Pending, SessionBus, SignalMatch, SystemBus,
    fdo, variant,
};
use portal_core::{PORTAL_PATH, RESPONSE_SUCCESS, RequestClose};
use zbus::message::Message;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

use super::helpers::{describe_flags, parse_inhibit_options};
use super::interface::{
    CreateMonitor, INHIBIT_IFACE, INHIBIT_VERSION, Inhibit, LockSession, LogindInhibit,
    PrepareForSleep, PrepareForShutdown, QueryEndResponse, SimulateStateChanged, StateChanged, UnlockSession,
};
use super::types::{InhibitEntry, MonitorSession, RequestHandle, SessionHandle, SessionState};

/// The Inhibit portal backend.
///
/// Handles two distinct request flows:
///
/// 1. **Inhibit** — long-lived request. The D-Bus reply is deferred until
///    `Request.Close` arrives on `handle`. This keeps the inhibition active.
///
/// 2. **CreateMonitor** — creates a monitoring session. Returns `response=0`
///    immediately. While the session is alive, `StateChanged` signals are sent
///    to it whenever the session state changes (screensaver, logout query, etc).
#[derive(State)]
pub struct InhibitBackend {
    proxy: DbusProxy<SessionBus>,
    system_proxy: DbusProxy<SystemBus>,

    /// Pending Inhibit calls: request-handle → (raw message, entry).
    /// The reply is held until Request.Close is received.
    pending: HashMap<RequestHandle, (Rc<Message>, InhibitEntry)>,

    /// In-flight calls to systemd-logind.
    logind_calls: Pending<LogindInhibit, RequestHandle>,

    /// Active monitoring sessions: session-handle → session info.
    monitors: HashMap<SessionHandle, MonitorSession>,
}

impl InhibitBackend {
    pub fn new(proxy: DbusProxy<SessionBus>, system_proxy: DbusProxy<SystemBus>) -> Self {
        system_proxy.subscribe::<PrepareForSleep>();
        system_proxy.subscribe::<PrepareForShutdown>();
        system_proxy.subscribe::<LockSession>();
        system_proxy.subscribe::<UnlockSession>();

        Self {
            proxy,
            system_proxy,
            pending: HashMap::new(),
            logind_calls: Pending::new(),
            monitors: HashMap::new(),
        }
    }

    /// Stash a raw Inhibit message, keyed by its request handle.
    fn stash_inhibit(
        &mut self,
        handle: &OwnedObjectPath,
        raw: &Rc<Message>,
        entry: InhibitEntry,
    ) -> RequestHandle {
        let key = handle.as_str().to_string();
        self.pending.insert(key.clone(), (Rc::clone(raw), entry));
        key
    }

    /// Release an inhibition: remove the entry (which closes the fd).
    fn release_inhibit(&mut self, handle: &str) {
        if let Some((_raw, entry)) = self.pending.remove(handle) {
            println!(
                "[inhibit-backend] Released inhibition handle={handle} app_id={} flags={}",
                entry.app_id,
                describe_flags(entry.flags)
            );
        }
    }

    /// Register a new monitoring session.
    fn add_monitor(&mut self, session_handle: &OwnedObjectPath, app_id: &str) {
        let key = session_handle.as_str().to_string();
        println!("[inhibit-backend] CreateMonitor: session={key} app_id={app_id}");
        self.monitors.insert(
            key.clone(),
            MonitorSession {
                session_handle: key,
                app_id: app_id.to_string(),
            },
        );
    }

    /// Remove a monitoring session (e.g. when its Session.Close is called).
    fn remove_monitor(&mut self, session_handle: &str) {
        if self.monitors.remove(session_handle).is_some() {
            println!("[inhibit-backend] Monitor session removed: session={session_handle}");
        }
    }

    /// Emit a `StateChanged` signal to all active monitoring sessions.
    pub fn broadcast_state_changed(&self, state: SessionState, screensaver_active: bool) {
        let mut vardict: HashMap<String, OwnedValue> = HashMap::new();

        vardict.insert(
            "screensaver-active".to_string(),
            OwnedValue::try_from(Value::Bool(screensaver_active)).unwrap(),
        );
        vardict.insert(
            "session-state".to_string(),
            OwnedValue::try_from(Value::U32(state.as_u32())).unwrap(),
        );

        for (session_handle, _monitor) in &self.monitors {
            let path = session_handle.as_str();
            // The signal arg is an OwnedObjectPath — parse from the session handle string.
            if let Ok(obj_path) = OwnedObjectPath::try_from(session_handle.clone()) {
                self.proxy
                    .emit::<StateChanged>(PORTAL_PATH, &(obj_path, vardict.clone()));
                println!(
                    "[inhibit-backend] StateChanged emitted: session={path} state={:?}",
                    state
                );
            }
        }
    }
}

pub fn inhibit_backend_module<S>() -> impl RegisteredModule<InhibitBackend, S> {
    Module::<InhibitBackend, _, _>::new()
        .on(
            |s: &mut InhibitBackend, ev: &DbusEvent<SessionBus>| -> Option<()> {
                match &ev.msg {
                    DbusMessage::Disconnected => return None,
                    _ => {}
                }

                // Inhibit
                // Long-lived request. The reply is deferred until Request.Close.
                // Per spec, no out-args are returned.
                if let Some(Ok(call)) = IncomingCall::<Inhibit>::try_from(&ev.msg) {
                    let (handle, app_id, parent_window, flags, options) = &call.args;
                    let opts = parse_inhibit_options(options);

                    println!(
                        "[inhibit-backend] Inhibit: app_id={app_id} handle={} flags={} reason={:?} window={parent_window}",
                        handle.as_str(),
                        describe_flags(*flags),
                        opts.reason
                    );

                    let entry = InhibitEntry {
                        handle: handle.as_str().to_string(),
                        app_id: app_id.clone(),
                        flags: *flags,
                        reason: opts.reason.clone(),
                        fd: None,
                    };

                    s.stash_inhibit(handle, call.raw(), entry);

                    // Call systemd-logind Inhibit
                    let mut what = Vec::new();
                    if flags & 1 != 0 {
                        what.push("shutdown");
                    }
                    if flags & 4 != 0 {
                        what.push("sleep");
                    }
                    if flags & 8 != 0 {
                        what.push("idle");
                    }
                    let what_str = if what.is_empty() {
                        "idle".to_string()
                    } else {
                        what.join(":")
                    };

                    let who = app_id.clone();
                    let why = opts.reason.clone().unwrap_or_else(|| "Application requested inhibition".to_string());
                    let mode = "block".to_string();

                    let handle_str = handle.as_str().to_string();
                    s.logind_calls.call(
                        &s.system_proxy,
                        &(what_str, who, why, mode),
                        handle_str,
                    );

                    // Do NOT reply here — inhibition lasts until Request.Close.
                    return None;
                }

                // CreateMonitor 
                // Immediately returns response=0 (success). While the session lives,
                // StateChanged signals will be emitted to it.
                if let Some(Ok(call)) = IncomingCall::<CreateMonitor>::try_from(&ev.msg) {
                    let (handle, session_handle, app_id, parent_window) = &call.args;
                    println!(
                        "[inhibit-backend] CreateMonitor: app_id={app_id} \
                         handle={} session={} window={parent_window}",
                        handle.as_str(),
                        session_handle.as_str()
                    );

                    s.add_monitor(session_handle, app_id);

                    // Reply immediately: (response: u32) = 0 (success).
                    s.proxy.reply(call.raw(), &(RESPONSE_SUCCESS,));
                    return None;
                }

                //  QueryEndResponse 
                // Application acknowledges a StateChanged with session-state=QueryEnd.
                // Must arrive within ~1 second of the signal.
                // TODO: Track application acknowledgement and notify the session manager/systemd before proceeding with session end.
                if let Some(Ok(call)) = IncomingCall::<QueryEndResponse>::try_from(&ev.msg) {
                    let (session_handle,) = &call.args;
                    println!(
                        "[inhibit-backend] QueryEndResponse: session={}",
                        session_handle.as_str()
                    );
                    call.respond(&s.proxy, &());
                    return None;
                }

                //  SimulateStateChanged 
                // Helper for manual simulation/testing of state changes.
                if let Some(Ok(call)) = IncomingCall::<SimulateStateChanged>::try_from(&ev.msg) {
                    let (state_u32, screensaver_active) = &call.args;
                    let state = match *state_u32 {
                        1 => SessionState::Running,
                        2 => SessionState::QueryEnd,
                        3 => SessionState::Ending,
                        _ => SessionState::Running,
                    };
                    println!(
                        "[inhibit-backend] SimulateStateChanged: state={:?} screensaver={}",
                        state,
                        screensaver_active
                    );
                    s.broadcast_state_changed(state, *screensaver_active);
                    call.respond(&s.proxy, &());
                    return None;
                }

                //  Request.Close 
                // Ends either an active inhibition (by request handle) or a monitor
                // session (by session handle). Both share this mechanism.
                if let Some(Ok(call)) = IncomingCall::<RequestClose>::try_from(&ev.msg) {
                    if let Some(handle) = &call.path {
                        call.respond(&s.proxy, &());
                        // Try to end an inhibition first.
                        if s.pending.contains_key(handle) {
                            println!(
                                "[inhibit-backend] Request.Close for inhibit handle={handle}"
                            );
                            s.release_inhibit(handle);
                        } else {
                            // Otherwise close the monitoring session.
                            println!(
                                "[inhibit-backend] Request.Close for monitor session={handle}"
                            );
                            s.remove_monitor(handle);
                        }
                    }
                    return None;
                }

                //  Properties 
                if fdo::route_properties(
                    &s.proxy,
                    &ev.msg,
                    INHIBIT_IFACE,
                    &["version"],
                    |access| match access {
                        fdo::PropAccess::Get("version") => {
                            fdo::PropReply::Value(variant(INHIBIT_VERSION))
                        }
                        fdo::PropAccess::Set("version", _) => fdo::PropReply::ReadOnly,
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
                            .is_some_and(|i| i.as_str() == INHIBIT_IFACE)
                    {
                        eprintln!(
                            "[inhibit-backend] FALLBACK: unknown method={:?} on Inhibit interface",
                            m.header().member()
                        );
                        s.proxy.reply_unknown_method(m);
                    }
                }

                None
            },
        )
        .on(
            |s: &mut InhibitBackend, ev: &DbusEvent<SystemBus>| -> Option<()> {
                match &ev.msg {
                    DbusMessage::Disconnected => {
                        println!("[inhibit-backend] System bus disconnected");
                        s.logind_calls.clear();
                        for (_handle, (_raw, entry)) in &mut s.pending {
                            entry.fd = None;
                        }
                        return None;
                    }
                    DbusMessage::Reconnected => {
                        println!("[inhibit-backend] System bus reconnected");
                        s.system_proxy.subscribe::<PrepareForSleep>();
                        s.system_proxy.subscribe::<PrepareForShutdown>();
                        s.system_proxy.subscribe::<LockSession>();
                        s.system_proxy.subscribe::<UnlockSession>();
                        return None;
                    }
                    _ => {}
                }

                // Handle systemd-logind Lock signal
                if let Some(Ok(_sig)) = SignalMatch::<LockSession>::try_from(&ev.msg) {
                    println!("[inhibit-backend] Received logind Lock signal");
                    s.broadcast_state_changed(SessionState::Running, true);
                    return None;
                }

                // Handle systemd-logind Unlock signal
                if let Some(Ok(_sig)) = SignalMatch::<UnlockSession>::try_from(&ev.msg) {
                    println!("[inhibit-backend] Received logind Unlock signal");
                    s.broadcast_state_changed(SessionState::Running, false);
                    return None;
                }

                // Handle systemd-logind PrepareForSleep signal
                if let Some(Ok(sig)) = SignalMatch::<PrepareForSleep>::try_from(&ev.msg) {
                    let (sleep_active,) = sig.args;
                    println!("[inhibit-backend] Received logind PrepareForSleep signal (active={sleep_active})");
                    if sleep_active {
                        s.broadcast_state_changed(SessionState::QueryEnd, true);
                    } else {
                        s.broadcast_state_changed(SessionState::Running, false);
                    }
                    return None;
                }

                // Handle systemd-logind PrepareForShutdown signal
                if let Some(Ok(sig)) = SignalMatch::<PrepareForShutdown>::try_from(&ev.msg) {
                    let (shutdown_active,) = sig.args;
                    println!("[inhibit-backend] Received logind PrepareForShutdown signal (active={shutdown_active})");
                    if shutdown_active {
                        s.broadcast_state_changed(SessionState::QueryEnd, true);
                    } else {
                        s.broadcast_state_changed(SessionState::Running, false);
                    }
                    return None;
                }

                if let Some((handle, res)) = s.logind_calls.resolve(&ev.msg) {
                    match res {
                        Ok(fd) => {
                            println!("[inhibit-backend] Logind Inhibit successful for handle={handle}");
                            if let Some((raw, entry)) = s.pending.get_mut(&handle) {
                                entry.fd = Some(fd);
                                // Reply success to the stashed Inhibit call now that inhibition is active
                                s.proxy.reply(raw, &());
                            }
                        }
                        Err(e) => {
                            eprintln!("[inhibit-backend] Logind Inhibit call failed for handle={handle}: {e}");
                            if let Some((raw, _entry)) = s.pending.remove(&handle) {
                                s.proxy.reply_error(
                                    &raw,
                                    "org.freedesktop.portal.Error.Failed",
                                    &format!("Logind inhibition failed: {e}"),
                                );
                            }
                        }
                    }
                }

                None
            },
        )
}

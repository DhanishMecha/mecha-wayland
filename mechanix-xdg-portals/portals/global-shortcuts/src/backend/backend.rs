use app::{RegisteredModule, prelude::*};
use dbus::{DbusEvent, DbusMessage, DbusProxy, IncomingCall, SessionBus, fdo, variant};
use portal_core::PORTAL_PATH;
use std::collections::HashMap;
use std::rc::Rc;
use zbus::message::Message;
use zbus::zvariant::{OwnedValue, Value};

use super::interface::{
    Activated, BindShortcuts, ConfigureShortcuts, CreateSession, Deactivated,
    GLOBAL_SHORTCUTS_IFACE, GLOBAL_SHORTCUTS_VERSION, ListShortcuts, ShortcutsChanged,
};
use super::types::{GlobalShortcutsOutcome, GlobalShortcutsRequest, GlobalShortcutsResponse};

pub struct GlobalShortcutsSession {
    pub app_id: String,
    pub shortcuts: Vec<(String, HashMap<String, OwnedValue>)>,
    pub closed: bool,
}

#[derive(State)]
pub struct GlobalShortcutsBackend {
    proxy: DbusProxy<SessionBus>,
    sessions: HashMap<String, GlobalShortcutsSession>,
    pending: HashMap<String, (Rc<Message>, String, Vec<(String, HashMap<String, OwnedValue>)>)>,
}

fn get_string_prop(map: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    map.get(key).and_then(|v| {
        if let Ok(s) = String::try_from(v.clone()) {
            Some(s)
        } else {
            let inner = Value::from(v.clone());
            if let Value::Value(boxed) = inner {
                String::try_from(*boxed).ok()
            } else {
                None
            }
        }
    })
}

impl GlobalShortcutsBackend {
    pub fn new(proxy: DbusProxy<SessionBus>) -> Self {
        Self {
            proxy,
            sessions: HashMap::new(),
            pending: HashMap::new(),
        }
    }

    fn finish_bind(&mut self, handle: &str, outcome: GlobalShortcutsOutcome) {
        let Some((raw, session_handle, shortcuts)) = self.pending.remove(handle) else {
            return;
        };
        match outcome {
            GlobalShortcutsOutcome::Granted => {
                let mut registered_shortcuts = Vec::new();
                for (shortcut_id, shortcut_props) in shortcuts {
                    let active_trigger =
                        get_string_prop(&shortcut_props, "preferred_trigger").unwrap_or_default();

                    let mut active_props = HashMap::new();
                    active_props.insert(
                        "trigger".to_string(),
                        OwnedValue::try_from(Value::Str(active_trigger.into())).unwrap(),
                    );
                    registered_shortcuts.push((shortcut_id.clone(), active_props));
                }

                if let Some(session) = self.sessions.get_mut(&session_handle) {
                    session.shortcuts = registered_shortcuts.clone();
                }

                let session_path =
                    zbus::zvariant::OwnedObjectPath::try_from(session_handle.clone()).unwrap();
                self.proxy.emit::<ShortcutsChanged>(
                    PORTAL_PATH,
                    &(session_path, registered_shortcuts.clone()),
                );

                // Simulate a mock shortcut press and release event for "play-pause" to verify signals work
                self.emit_shortcut_action(&session_handle, "play-pause", true);
                self.emit_shortcut_action(&session_handle, "play-pause", false);

                let shortcuts_val =
                    OwnedValue::try_from(Value::from(registered_shortcuts)).unwrap();
                let mut results = HashMap::new();
                results.insert("shortcuts".to_string(), shortcuts_val);

                self.proxy.reply(&raw, &(0u32, results));
            }
            GlobalShortcutsOutcome::Denied => {
                self.proxy
                    .reply(&raw, &(1u32, HashMap::<String, OwnedValue>::new()));
            }
        }
    }

    /// Emit the Activated or Deactivated D-Bus signals for a registered shortcut
    pub fn emit_shortcut_action(
        &self,
        session_handle: &str,
        shortcut_id: &str,
        activated: bool,
    ) {
        let session_path =
            zbus::zvariant::OwnedObjectPath::try_from(session_handle.to_string()).unwrap();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;
        let options = HashMap::<String, OwnedValue>::new();

        if activated {
            println!("[global-shortcuts-backend] Emitting Activated for session={}, shortcut={}", session_handle, shortcut_id);
            self.proxy.emit::<Activated>(
                PORTAL_PATH,
                &(session_path, shortcut_id.to_string(), timestamp, options),
            );
        } else {
            println!("[global-shortcuts-backend] Emitting Deactivated for session={}, shortcut={}", session_handle, shortcut_id);
            self.proxy.emit::<Deactivated>(
                PORTAL_PATH,
                &(session_path, shortcut_id.to_string(), timestamp, options),
            );
        }
    }

    /// TODO: Integrate with the compositor's keyboard event loop to listen for pressed key combinations,
    /// matching them against the session's registered triggers, and calling `emit_shortcut_action` accordingly.
    pub fn listen_to_compositor_events(&self) {
        // Placeholder for compositor event loop integration
    }
}

pub fn global_shortcuts_backend_module<S>() -> impl RegisteredModule<GlobalShortcutsBackend, S> {
    Module::<GlobalShortcutsBackend, _, _>::new()
        .on(|s: &mut GlobalShortcutsBackend, resp: &GlobalShortcutsResponse| {
            s.finish_bind(&resp.handle, resp.outcome.clone());
        })
        .on(
            |s: &mut GlobalShortcutsBackend, ev: &DbusEvent<SessionBus>| -> Option<GlobalShortcutsRequest> {
                // CreateSession()
                if let Some(Ok(call)) = IncomingCall::<CreateSession>::try_from(&ev.msg) {
                    let (_handle, session_handle, app_id, _options) = &call.args;
                    let session_handle_str = session_handle.as_str().to_string();
                    println!("[global-shortcuts-backend] CreateSession requested for app_id={app_id}, session_handle={session_handle_str}");

                    let session = GlobalShortcutsSession {
                        app_id: app_id.clone(),
                        shortcuts: Vec::new(),
                        closed: false,
                    };
                    s.sessions.insert(session_handle_str, session);

                    call.respond(&s.proxy, &(0u32, HashMap::new()));
                    return None;
                }

                // BindShortcuts()
                if let Some(Ok(call)) = IncomingCall::<BindShortcuts>::try_from(&ev.msg) {
                    let (handle, session_handle, shortcuts, parent_window, _options) = &call.args;
                    let session_handle_str = session_handle.as_str().to_string();
                    println!(
                        "[global-shortcuts-backend] BindShortcuts requested: shortcuts_count={}, parent_window={}, session_handle={}",
                        shortcuts.len(),
                        parent_window,
                        session_handle_str
                    );

                    let Some(session) = s.sessions.get_mut(&session_handle_str) else {
                        eprintln!("[global-shortcuts-backend] BindShortcuts: session not found: {session_handle_str}");
                        call.respond(&s.proxy, &(1u32, HashMap::<String, OwnedValue>::new()));
                        return None;
                    };

                    session.shortcuts = shortcuts.clone();

                    let mut shortcuts_list = Vec::new();
                    for (shortcut_id, props) in shortcuts {
                        let description = get_string_prop(props, "description")
                            .unwrap_or_else(|| shortcut_id.clone());
                        let preferred_trigger = get_string_prop(props, "preferred_trigger")
                            .unwrap_or_else(|| "none".to_string());
                        shortcuts_list.push((description, preferred_trigger));
                    }

                    let handle_str = handle.as_str().to_string();
                    s.pending.insert(handle_str.clone(), (call.raw().clone(), session_handle_str.clone(), shortcuts.clone()));

                    return Some(GlobalShortcutsRequest::BindDialog {
                        handle: handle_str,
                        app_id: session.app_id.clone(),
                        shortcuts: shortcuts_list,
                    });
                }

                // ListShortcuts()
                if let Some(Ok(call)) = IncomingCall::<ListShortcuts>::try_from(&ev.msg) {
                    let (_handle, session_handle) = &call.args;
                    let session_handle_str = session_handle.as_str().to_string();
                    println!("[global-shortcuts-backend] ListShortcuts requested for session_handle={session_handle_str}");

                    let shortcuts = s.sessions.get(&session_handle_str)
                        .map(|session| session.shortcuts.clone())
                        .unwrap_or_default();

                    let shortcuts_val = OwnedValue::try_from(Value::from(shortcuts)).unwrap();
                    let mut results = HashMap::new();
                    results.insert("shortcuts".to_string(), shortcuts_val);

                    call.respond(&s.proxy, &(0u32, results));
                    return None;
                }

                // ConfigureShortcuts()
                if let Some(Ok(call)) = IncomingCall::<ConfigureShortcuts>::try_from(&ev.msg) {
                    let (session_handle, parent_window, _options) = &call.args;
                    let session_handle_str = session_handle.as_str().to_string();
                    println!(
                        "[global-shortcuts-backend] ConfigureShortcuts requested: session_handle={}, parent_window={}",
                        session_handle_str,
                        parent_window
                    );

                    let Some(session) = s.sessions.get(&session_handle_str) else {
                        eprintln!("[global-shortcuts-backend] ConfigureShortcuts: session not found: {session_handle_str}");
                        call.respond(&s.proxy, &());
                        return None;
                    };

                    let mut shortcuts_list = Vec::new();
                    for (shortcut_id, props) in &session.shortcuts {
                        let description = get_string_prop(props, "description")
                            .unwrap_or_else(|| shortcut_id.clone());
                        let trigger = get_string_prop(props, "trigger")
                            .unwrap_or_else(|| "none".to_string());
                        shortcuts_list.push((description, trigger));
                    }

                    let handle_str = format!("{}/configure", session_handle_str);

                    call.respond(&s.proxy, &());

                    return Some(GlobalShortcutsRequest::ConfigureDialog {
                        handle: handle_str,
                        app_id: session.app_id.clone(),
                        shortcuts: shortcuts_list,
                    });
                }

                // Request.Close()
                if let Some(Ok(call)) = IncomingCall::<portal_core::RequestClose>::try_from(&ev.msg) {
                    if let Some(handle) = &call.path {
                        call.respond(&s.proxy, &());
                        if s.pending.contains_key(handle) {
                            let h = handle.clone();
                            s.finish_bind(&h, GlobalShortcutsOutcome::Denied);
                            return Some(GlobalShortcutsRequest::Close { handle: h });
                        }
                    }
                    return None;
                }

                // Properties
                if fdo::route_properties(&s.proxy, &ev.msg, GLOBAL_SHORTCUTS_IFACE, &["version"], |access| {
                    match access {
                        fdo::PropAccess::Get("version") => {
                            fdo::PropReply::Value(variant(GLOBAL_SHORTCUTS_VERSION))
                        }
                        fdo::PropAccess::Set("version", _) => fdo::PropReply::ReadOnly,
                        _ => fdo::PropReply::Unknown,
                    }
                }) {
                    return None;
                }

                // Fallback
                if let DbusMessage::Call(m) = &ev.msg {
                    if m.header().path().is_some_and(|p| p.as_str() == PORTAL_PATH)
                        && m.header()
                            .interface()
                            .is_some_and(|i| i.as_str() == GLOBAL_SHORTCUTS_IFACE)
                    {
                        eprintln!("[global-shortcuts-backend] FALLBACK: unknown method on GlobalShortcuts interface");
                        s.proxy.reply_unknown_method(m);
                    }
                }

                None
            },
        )
}

use std::collections::HashMap;
use std::rc::Rc;

use app::{prelude::*, RegisteredModule};
use dbus::{DbusEvent, DbusMessage, DbusProxy, IncomingCall, SessionBus};
use zbus::message::Message;
use zbus::zvariant::{OwnedValue, Value};

use super::interface::{
    BACKGROUND_IFACE, EnableAutostart, GetAppState, NotifyBackground,
    RunningApplicationsChanged,
};
use super::types::{BackgroundOutcome, BackgroundRequest, BackgroundResponse, RequestHandle};
use portal_core::{PORTAL_PATH, RequestClose, RESPONSE_SUCCESS};

#[derive(State)]
pub struct BackgroundBackend {
    proxy: DbusProxy<SessionBus>,
    pending: HashMap<RequestHandle, Rc<Message>>,
    // Tracks state of each app: 0 = Background, 1 = Running, 2 = Active
    app_states: HashMap<String, u32>,
    // Mapping of handle -> app_id
    handle_to_app: HashMap<RequestHandle, String>,
}

impl BackgroundBackend {
    pub fn new(proxy: DbusProxy<SessionBus>) -> Self {
        Self {
            proxy,
            pending: HashMap::new(),
            app_states: HashMap::new(),
            handle_to_app: HashMap::new(),
        }
    }

    fn stash_pending(
        &mut self,
        handle: &zbus::zvariant::OwnedObjectPath,
        raw: &Rc<Message>,
        app_id: &str,
    ) -> RequestHandle {
        let handle_str = handle.as_str().to_string();
        self.pending.insert(handle_str.clone(), Rc::clone(raw));
        self.handle_to_app.insert(handle_str.clone(), app_id.to_string());
        handle_str
    }

    fn finish_notify(&mut self, handle: &str, outcome: BackgroundOutcome) {
        let Some(raw) = self.pending.remove(handle) else {
            return;
        };

        let app_id_opt = self.handle_to_app.remove(handle);
        let choice = outcome.to_u32();

        // If the request was allowed, update the tracked application state to 0 (Background)
        if choice == 1 || choice == 2 {
            if let Some(app_id) = app_id_opt {
                self.set_app_state(app_id, 0);
            }
        }

        let mut results = HashMap::new();
        results.insert(
            "result".to_string(),
            OwnedValue::try_from(Value::U32(choice)).unwrap(),
        );

        self.proxy.reply(&raw, &(RESPONSE_SUCCESS, results));
    }

    pub fn set_app_state(&mut self, _app_id: String, _state: u32) {
        // TODO: Emitting RunningApplicationsChanged event trigger so the client can listen and call GetAppState again.
        println!("[background-backend] AppState changed: emitting RunningApplicationsChanged");
        self.proxy.emit::<RunningApplicationsChanged>(PORTAL_PATH, &());
    }
}

pub fn background_backend_module<S>() -> impl RegisteredModule<BackgroundBackend, S> {
    Module::<BackgroundBackend, _, _>::new()
        .on(|s: &mut BackgroundBackend, done: &BackgroundResponse| {
            s.finish_notify(&done.handle, done.outcome.clone());
        })
        .on(
            |s: &mut BackgroundBackend, ev: &DbusEvent<SessionBus>| -> Option<BackgroundRequest> {
                // GetAppState
                // TODO: Integrate with window-manager to get live window list for app states instead of tracking locally
                if let Some(Ok(call)) = IncomingCall::<GetAppState>::try_from(&ev.msg) {
                    println!("[background-backend] GetAppState requested");
                    let mut apps = HashMap::new();
                    for (app_id, state) in &s.app_states {
                        apps.insert(
                            app_id.clone(),
                            OwnedValue::try_from(Value::U32(*state)).unwrap(),
                        );
                    }
                    call.respond(&s.proxy, &(apps,));
                    return None;
                }

                // NotifyBackground
                // TODO: Store background authorization choices persistently (e.g. to a config file) so the user doesn't get prompted every time the app starts
                if let Some(Ok(call)) = IncomingCall::<NotifyBackground>::try_from(&ev.msg) {
                    let (handle, app_id, name) = &call.args;
                    println!(
                        "[background-backend] NotifyBackground: app_id={app_id} name={name} handle={}",
                        handle.as_str()
                    );
                    
                    let handle_str = s.stash_pending(handle, call.raw(), app_id);
                    return Some(BackgroundRequest::NotifyBackground {
                        handle: handle_str,
                        app_id: app_id.clone(),
                        name: name.clone(),
                    });
                }

                // EnableAutostart (deprecated)
                // TODO: Handle autostart by writing desktop files to autostart directory (e.g. ~/.config/autostart/)
                if let Some(Ok(call)) = IncomingCall::<EnableAutostart>::try_from(&ev.msg) {
                    let (app_id, enable, commandline, flags) = &call.args;
                    println!(
                        "[background-backend] EnableAutostart (deprecated): app_id={app_id} enable={enable} cmd={commandline:?} flags={flags}"
                    );
                    call.respond(&s.proxy, &(true,));
                    return None;
                }

                // Request.Close
                if let Some(Ok(call)) = IncomingCall::<RequestClose>::try_from(&ev.msg) {
                    if let Some(handle) = &call.path {
                        call.respond(&s.proxy, &());
                        if s.pending.contains_key(handle) {
                            let handle = handle.clone();
                            s.finish_notify(&handle, BackgroundOutcome::Forbid);
                            return Some(BackgroundRequest::Close { handle });
                        }
                    }
                    return None;
                }

                // Fallback
                if let DbusMessage::Call(m) = &ev.msg {
                    if m.header().path().is_some_and(|p| p.as_str() == PORTAL_PATH)
                        && m.header()
                            .interface()
                            .is_some_and(|i| i.as_str() == BACKGROUND_IFACE)
                    {
                        eprintln!("[background-backend] FALLBACK: unknown method on Background interface");
                        s.proxy.reply_unknown_method(m);
                    }
                }

                None
            },
        )
}

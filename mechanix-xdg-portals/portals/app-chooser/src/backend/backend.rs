use std::collections::HashMap;
use std::rc::Rc;

use app::{RegisteredModule, prelude::*};
use dbus::{DbusEvent, DbusMessage, DbusProxy, IncomingCall, SessionBus, fdo, variant};
use zbus::message::Message;

use super::interface::{APP_CHOOSER_IFACE, APP_CHOOSER_VERSION, ChooseApplication, UpdateChoices};
use super::types::{
    AppChooserOutcome, AppChooserRequest, AppChooserResponse, AppChooserResults, RequestHandle,
};
use portal_core::{PORTAL_PATH, RESPONSE_CANCELLED, RESPONSE_SUCCESS, RequestClose};

#[derive(State)]
pub struct AppChooserBackend {
    proxy: DbusProxy<SessionBus>,
    pending: HashMap<RequestHandle, Rc<Message>>,
}

impl AppChooserBackend {
    pub fn new(proxy: DbusProxy<SessionBus>) -> Self {
        Self {
            proxy,
            pending: HashMap::new(),
        }
    }

    fn stash_pending(
        &mut self,
        handle: &zbus::zvariant::OwnedObjectPath,
        raw: &Rc<Message>,
    ) -> RequestHandle {
        let handle_str = handle.as_str().to_string();
        self.pending.insert(handle_str.clone(), Rc::clone(raw));
        handle_str
    }

    fn finish_dialog(&mut self, handle: &str, outcome: AppChooserOutcome) {
        let Some(raw) = self.pending.remove(handle) else {
            return;
        };
        // TODO: handling with the default configuration of app choices (e.g. updating system MIME associations in mimeapps.list)
        let (response, results) = match outcome {
            AppChooserOutcome::Chosen(ref app_id) => (
                RESPONSE_SUCCESS,
                AppChooserResults {
                    choice: Some(app_id.clone()),
                    // TODO: mint a fresh xdg_activation_v1 token for the chosen app and
                    // return it here. This requires a Wayland compositor connection
                    // (xdg_activation_v1 global) in AppChooserBackend, which is not yet
                    // available. Without it the caller cannot guarantee the launched app
                    // will be allowed to raise its window by the compositor.
                    activation_token: None,
                },
            ),

            AppChooserOutcome::Cancelled => (
                RESPONSE_CANCELLED,
                AppChooserResults {
                    choice: None,
                    activation_token: None,
                },
            ),
        };
        self.proxy.reply(&raw, &(response, results));
    }
}

pub fn app_chooser_backend_module<S>() -> impl RegisteredModule<AppChooserBackend, S> {
    Module::<AppChooserBackend, _, _>::new()
        // Handle the UI response: reply to D-Bus and remove from pending.
        .on(|s: &mut AppChooserBackend, done: &AppChooserResponse| {
            s.finish_dialog(&done.handle, done.outcome.clone());
        })
        // Handle incoming D-Bus messages.
        .on(
            |s: &mut AppChooserBackend, ev: &DbusEvent<SessionBus>| -> Option<AppChooserRequest> {
                match &ev.msg {
                    DbusMessage::Disconnected => {
                        s.pending.clear();
                        return None;
                    }
                    _ => {}
                }

                // ChooseApplication → open the picker dialog.
                if let Some(Ok(call)) = IncomingCall::<ChooseApplication>::try_from(&ev.msg) {
                    let (handle, app_id, _parent, choices, options) = &call.args;
                    println!(
                        "[app-chooser] ChooseApplication: app_id={app_id} choices={:?} handle={}",
                        choices,
                        handle.as_str()
                    );
                    return Some(AppChooserRequest::ChooseApplication {
                        handle: s.stash_pending(handle, call.raw()),
                        app_id: app_id.clone(),
                        choices: choices.clone(),
                        last_choice: options.last_choice.clone(),
                        content_type: options.content_type.clone(),
                        uri: options.uri.clone(),
                        filename: options.filename.clone(),
                    });
                }

                // UpdateChoices → refresh the list while dialog is open.
                if let Some(Ok(call)) = IncomingCall::<UpdateChoices>::try_from(&ev.msg) {
                    let (handle, choices) = &call.args;
                    let handle_str = handle.as_str().to_string();
                    if s.pending.contains_key(&handle_str) {
                        // Reply immediately — UpdateChoices has no meaningful return value.
                        call.respond(&s.proxy, &());
                        return Some(AppChooserRequest::UpdateChoices {
                            handle: handle_str,
                            choices: choices.clone(),
                        });
                    }
                    return None;
                }

                // Request.Close → cancel.
                if let Some(Ok(call)) = IncomingCall::<RequestClose>::try_from(&ev.msg) {
                    if let Some(handle) = &call.path {
                        // Always respond to Close() so the caller never hangs.
                        call.respond(&s.proxy, &());
                        if s.pending.contains_key(handle) {
                            let handle = handle.clone();
                            s.finish_dialog(&handle, AppChooserOutcome::Cancelled);
                            return Some(AppChooserRequest::Close { handle });
                        }
                    }
                    return None;
                }

                // Properties: read-only `version`.
                if fdo::route_properties(
                    &s.proxy,
                    &ev.msg,
                    APP_CHOOSER_IFACE,
                    &["version"],
                    |prop| match prop {
                        fdo::PropAccess::Get("version") => {
                            fdo::PropReply::Value(variant(APP_CHOOSER_VERSION))
                        }
                        fdo::PropAccess::Set("version", _) => fdo::PropReply::ReadOnly,
                        _ => fdo::PropReply::Unknown,
                    },
                ) {
                    return None;
                }

                // Fallback: unknown method on our interface.
                if let DbusMessage::Call(m) = &ev.msg {
                    if m.header().path().is_some_and(|p| p.as_str() == PORTAL_PATH)
                        && m.header()
                            .interface()
                            .is_some_and(|i| i.as_str() == APP_CHOOSER_IFACE)
                    {
                        eprintln!("[app-chooser-backend] FALLBACK: unknown_method for AppChooser");
                        s.proxy.reply_unknown_method(m);
                    }
                }
                None
            },
        )
}

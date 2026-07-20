use std::collections::HashMap;
use std::rc::Rc;

use app::{prelude::*, RegisteredModule};
use dbus::{fdo, variant, DbusEvent, DbusMessage, DbusProxy, IncomingCall, SessionBus};
use zbus::message::Message;

use super::interface::{AccessDialog, ACCESS_IFACE, ACCESS_VERSION};
use super::types::{AccessOutcome, AccessRequest, AccessResponse, AccessResults, RequestHandle};
use portal_core::{RequestClose, PORTAL_PATH, RESPONSE_CANCELLED, RESPONSE_SUCCESS};

#[derive(State)]
pub struct AccessBackend {
    proxy: DbusProxy<SessionBus>,
    pending: HashMap<RequestHandle, Rc<Message>>,
}

impl AccessBackend {
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

    fn finish_dialog(&mut self, handle: &str, outcome: AccessOutcome) {
        let Some(raw) = self.pending.remove(handle) else {
            return;
        };
        let (response, results) = match outcome {
            AccessOutcome::Granted => (RESPONSE_SUCCESS, AccessResults { choices: None }),
            AccessOutcome::Denied => (RESPONSE_CANCELLED, AccessResults { choices: None }),
        };
        self.proxy.reply(&raw, &(response, results));
    }
}

pub fn access_backend_module<S>() -> impl RegisteredModule<AccessBackend, S> {
    Module::<AccessBackend, _, _>::new()
        .on(|s: &mut AccessBackend, done: &AccessResponse| {
            s.finish_dialog(&done.handle, done.outcome.clone());
        })
        .on(
            |s: &mut AccessBackend, ev: &DbusEvent<SessionBus>| -> Option<AccessRequest> {
                // AccessDialog -> open the dialog.
                match IncomingCall::<AccessDialog>::try_from(&ev.msg) {
                    Some(Ok(call)) => {
                        let (handle, app_id, _parent, title, subtitle, body, options) = &call.args;
                        return Some(AccessRequest::AccessDialog {
                            handle: s.stash_pending(handle, call.raw()),
                            app_id: app_id.clone(),
                            title: title.clone(),
                            subtitle: subtitle.clone(),
                            body: body.clone(),
                            options: options.clone(),
                        });
                    }
                    Some(Err(e)) => {
                        eprintln!("[access-backend] AccessDialog deserialization error: {e}");
                    }
                    None => {}
                }

                // Request.Close -> cancel.
                if let Some(Ok(call)) = IncomingCall::<RequestClose>::try_from(&ev.msg) {
                    if let Some(handle) = &call.path {
                        if s.pending.contains_key(handle) {
                            call.respond(&s.proxy, &());
                            let handle = handle.clone();
                            s.finish_dialog(&handle, AccessOutcome::Denied);
                            return Some(AccessRequest::Close { handle });
                        }
                    }
                    return None;
                }

                // Properties: read-only `version`.
                if fdo::route_properties(&s.proxy, &ev.msg, ACCESS_IFACE, &["version"], |access| {
                    match access {
                        fdo::PropAccess::Get("version") => {
                            fdo::PropReply::Value(variant(ACCESS_VERSION))
                        }
                        fdo::PropAccess::Set("version", _) => fdo::PropReply::ReadOnly,
                        _ => fdo::PropReply::Unknown,
                    }
                }) {
                    return None;
                }

                // Fallback: unknown method on our interface only.
                if let DbusMessage::Call(m) = &ev.msg {
                    if m.header().path().is_some_and(|p| p.as_str() == PORTAL_PATH)
                        && m.header()
                            .interface()
                            .is_some_and(|i| i.as_str() == ACCESS_IFACE)
                    {
                        eprintln!(
                            "[access-backend] FALLBACK: replying unknown_method for AccessDialog"
                        );
                        s.proxy.reply_unknown_method(m);
                    }
                }
                None
            },
        )
}

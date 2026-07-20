use std::collections::HashMap;
use std::rc::Rc;

use app::{prelude::*, RegisteredModule};
use dbus::{fdo, variant, DbusEvent, DbusMessage, DbusProxy, IncomingCall, SessionBus};
use zbus::message::Message;

use super::interface::{PickColor, Screenshot, SCREENSHOT_IFACE, SCREENSHOT_VERSION};
use super::types::{
    PickColorResults, RequestHandle, RequestKind, ScreenshotOutcome, ScreenshotRequest,
    ScreenshotResponse, ScreenshotResults,
};
use portal_core::{RequestClose, PORTAL_PATH, RESPONSE_CANCELLED, RESPONSE_SUCCESS};

#[derive(State)]
pub struct ScreenshotBackend {
    proxy: DbusProxy<SessionBus>,
    pending: HashMap<RequestHandle, (Rc<Message>, RequestKind)>,
}

impl ScreenshotBackend {
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
        kind: RequestKind,
    ) -> RequestHandle {
        let handle_str = handle.as_str().to_string();
        self.pending
            .insert(handle_str.clone(), (Rc::clone(raw), kind));
        handle_str
    }

    fn finish(&mut self, handle: &str, outcome: ScreenshotOutcome) {
        let Some((raw, kind)) = self.pending.remove(handle) else {
            return;
        };
        match (outcome, kind) {
            (ScreenshotOutcome::Granted, RequestKind::Screenshot) => {
                // TODO: Implement real capture via wlr-screencopy.
                let results = ScreenshotResults {
                    uri: Some("file:///tmp/mechanix-screenshot.png".to_string()),
                };
                self.proxy.reply(&raw, &(RESPONSE_SUCCESS, results));
            }
            (ScreenshotOutcome::Granted, RequestKind::PickColor) => {
                // TODO: Implement real colour picking via compositor protocol.
                let results = PickColorResults {
                    color: Some((0.5, 0.5, 0.5)),
                };
                self.proxy.reply(&raw, &(RESPONSE_SUCCESS, results));
            }
            (ScreenshotOutcome::Denied, RequestKind::Screenshot) => {
                self.proxy
                    .reply(&raw, &(RESPONSE_CANCELLED, ScreenshotResults::default()));
            }
            (ScreenshotOutcome::Denied, RequestKind::PickColor) => {
                self.proxy
                    .reply(&raw, &(RESPONSE_CANCELLED, PickColorResults::default()));
            }
        }
    }
}

pub fn screenshot_backend_module<S>() -> impl RegisteredModule<ScreenshotBackend, S> {
    Module::<ScreenshotBackend, _, _>::new()
        .on(|s: &mut ScreenshotBackend, done: &ScreenshotResponse| {
            s.finish(&done.handle, done.outcome.clone());
        })
        .on(
            |s: &mut ScreenshotBackend, ev: &DbusEvent<SessionBus>| -> Option<ScreenshotRequest> {
                // --- Screenshot() -------------------------------------------
                match IncomingCall::<Screenshot>::try_from(&ev.msg) {
                    Some(Ok(call)) => {
                        let (handle, app_id, _parent, options) = &call.args;
                        let interactive = options.interactive.unwrap_or(false);
                        let target = options.target;
                        println!(
                            "[screenshot-backend] Screenshot requested by app={app_id} \
                             interactive={interactive} target={target:?}"
                        );
                        return Some(ScreenshotRequest::Screenshot {
                            handle: s.stash_pending(handle, call.raw(), RequestKind::Screenshot),
                            app_id: app_id.clone(),
                            interactive,
                            target,
                        });
                    }
                    Some(Err(e)) => {
                        eprintln!("[screenshot-backend] Screenshot deserialization error: {e}");
                    }
                    None => {}
                }

                // --- PickColor() --------------------------------------------
                match IncomingCall::<PickColor>::try_from(&ev.msg) {
                    Some(Ok(call)) => {
                        let (handle, app_id, _parent, _options) = &call.args;
                        println!("[screenshot-backend] PickColor requested by app={app_id}");
                        return Some(ScreenshotRequest::PickColor {
                            handle: s.stash_pending(handle, call.raw(), RequestKind::PickColor),
                            app_id: app_id.clone(),
                        });
                    }
                    Some(Err(e)) => {
                        eprintln!("[screenshot-backend] PickColor deserialization error: {e}");
                    }
                    None => {}
                }

                // --- Request.Close() ----------------------------------------
                if let Some(Ok(call)) = IncomingCall::<RequestClose>::try_from(&ev.msg) {
                    if let Some(handle) = &call.path {
                        if s.pending.contains_key(handle) {
                            call.respond(&s.proxy, &());
                            let h = handle.clone();
                            s.finish(&h, ScreenshotOutcome::Denied);
                            return Some(ScreenshotRequest::Close { handle: h });
                        }
                    }
                    return None;
                }

                // --- Properties: version ------------------------------------
                if fdo::route_properties(
                    &s.proxy,
                    &ev.msg,
                    SCREENSHOT_IFACE,
                    &["version", "AvailableTargets"],
                    |access| match access {
                        fdo::PropAccess::Get("version") => {
                            fdo::PropReply::Value(variant(SCREENSHOT_VERSION))
                        }
                        fdo::PropAccess::Get("AvailableTargets") => {
                            // 1 (Screen) | 2 (Window) | 4 (Area) | 8 (Active Window) = 15
                            fdo::PropReply::Value(variant(15u32))
                        }
                        fdo::PropAccess::Set("version", _)
                        | fdo::PropAccess::Set("AvailableTargets", _) => fdo::PropReply::ReadOnly,
                        _ => fdo::PropReply::Unknown,
                    },
                ) {
                    return None;
                }

                // --- Fallback: unknown method on our interface ---------------
                if let DbusMessage::Call(m) = &ev.msg {
                    if m.header().path().is_some_and(|p| p.as_str() == PORTAL_PATH)
                        && m.header()
                            .interface()
                            .is_some_and(|i| i.as_str() == SCREENSHOT_IFACE)
                    {
                        eprintln!(
                            "[screenshot-backend] FALLBACK: unknown_method on Screenshot interface"
                        );
                        s.proxy.reply_unknown_method(m);
                    }
                }

                None
            },
        )
}

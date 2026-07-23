use std::collections::HashMap;
use std::rc::Rc;

use app::{ prelude::*, RegisteredModule };
use dbus::{ fdo, variant, DbusEvent, DbusMessage, DbusProxy, IncomingCall, SessionBus };
use zbus::message::Message;

use super::interface::{ OpenFile, SaveFile, SaveFiles, FILECHOOSER_IFACE, FILECHOOSER_VERSION };
use super::types::{
    FileChooserOutcome,
    FileChooserRequest,
    FileChooserResponse,
    FileChooserResults,
    RequestHandle,
};
use portal_core::{ RequestClose, PORTAL_PATH, RESPONSE_CANCELLED, RESPONSE_SUCCESS };

#[derive(State)]
pub struct FileChooserBackend {
    proxy: DbusProxy<SessionBus>,
    pending: HashMap<RequestHandle, Rc<Message>>,
}

impl FileChooserBackend {
    pub fn new(proxy: DbusProxy<SessionBus>) -> Self {
        Self {
            proxy,
            pending: HashMap::new(),
        }
    }

    fn finish_dialog(&mut self, handle: &str, outcome: FileChooserOutcome) {
        let Some(raw) = self.pending.remove(handle) else {
            return;
        };
        let (response, results) = match outcome {
            FileChooserOutcome::Selected(uris) => {
                (RESPONSE_SUCCESS, FileChooserResults { uris: Some(uris) })
            }
            FileChooserOutcome::Cancelled => {
                (RESPONSE_CANCELLED, FileChooserResults { uris: None })
            }
        };
        self.proxy.reply(&raw, &(response, results));
    }

    fn stash_pending(
        &mut self,
        handle: &zbus::zvariant::OwnedObjectPath,
        raw: &Rc<Message>
    ) -> RequestHandle {
        let handle_str = handle.as_str().to_string();
        self.pending.insert(handle_str.clone(), Rc::clone(raw));
        handle_str
    }
}

// --- Module registration -----------------------------------------------------
pub fn filechooser_module<S>() -> impl RegisteredModule<FileChooserBackend, S> {
    Module::<FileChooserBackend, _, _>
        ::new()
        .on(|s: &mut FileChooserBackend, done: &FileChooserResponse| {
            s.finish_dialog(&done.handle, done.outcome.clone());
        })
        .on(
            |s: &mut FileChooserBackend, ev: &DbusEvent<SessionBus>| -> Option<FileChooserRequest> {
                match &ev.msg {
                    DbusMessage::Disconnected => {
                        s.pending.clear();
                        return None;
                    }
                    _ => {}
                }

                // OpenFile
                if let Some(Ok(call)) = IncomingCall::<OpenFile>::try_from(&ev.msg) {
                    let (handle, app_id, _parent, title, _options) = &call.args;
                    println!("[file-chooser] OpenFile: app_id={app_id} title='{title}' handle={}", handle.as_str());
                    return Some(FileChooserRequest::OpenFile {
                        handle: s.stash_pending(handle, call.raw()),
                        title: title.clone(),
                        options: _options.clone(),
                    });
                }

                // SaveFile
                if let Some(Ok(call)) = IncomingCall::<SaveFile>::try_from(&ev.msg) {
                    let (handle, app_id, _parent, title, _options) = &call.args;
                    println!("[file-chooser] SaveFile: app_id={app_id} title='{title}' handle={}", handle.as_str());
                    return Some(FileChooserRequest::SaveFile {
                        handle: s.stash_pending(handle, call.raw()),
                        title: title.clone(),
                        options: _options.clone(),
                    });
                }

                // SaveFiles
                if let Some(Ok(call)) = IncomingCall::<SaveFiles>::try_from(&ev.msg) {
                    let (handle, app_id, _parent, title, _options) = &call.args;
                    println!("[file-chooser] SaveFiles: app_id={app_id} title='{title}' handle={}", handle.as_str());
                    return Some(FileChooserRequest::SaveFiles {
                        handle: s.stash_pending(handle, call.raw()),
                        title: title.clone(),
                        options: _options.clone(),
                    });
                }

                // Request.Close -> cancel
                if let Some(Ok(call)) = IncomingCall::<RequestClose>::try_from(&ev.msg) {
                    if let Some(handle) = &call.path {
                        if s.pending.contains_key(handle) {
                            call.respond(&s.proxy, &());
                            let handle = handle.clone();
                            s.finish_dialog(&handle, FileChooserOutcome::Cancelled);
                            return Some(FileChooserRequest::Close { handle });
                        }
                    }
                }

                // Properties: read-only `version`.
                if
                    fdo::route_properties(
                        &s.proxy,
                        &ev.msg,
                        FILECHOOSER_IFACE,
                        &["version"],
                        |access| {
                            match access {
                                fdo::PropAccess::Get("version") => {
                                    fdo::PropReply::Value(variant(FILECHOOSER_VERSION))
                                }
                                fdo::PropAccess::Set("version", _) => fdo::PropReply::ReadOnly,
                                _ => fdo::PropReply::Unknown,
                            }
                        }
                    )
                {
                    return None;
                }

                // Fallback: unknown method on our interface only.
                if let DbusMessage::Call(m) = &ev.msg {
                    if
                        m
                            .header()
                            .path()
                            .is_some_and(|p| p.as_str() == PORTAL_PATH) &&
                        m
                            .header()
                            .interface()
                            .is_some_and(|i| i.as_str() == FILECHOOSER_IFACE)
                    {
                        s.proxy.reply_unknown_method(m);
                    }
                }

                None
            }
        )
}

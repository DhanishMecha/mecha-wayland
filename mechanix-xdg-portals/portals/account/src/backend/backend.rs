use std::collections::HashMap;
use std::rc::Rc;

use app::{prelude::*, RegisteredModule};
use dbus::{fdo, variant, DbusEvent, DbusMessage, DbusProxy, IncomingCall, SessionBus, SystemBus, Pending};
use zbus::message::Message;

use super::interface::{
    FindUserById, GetUserProperties, GetUserInformation, ACCOUNT_IFACE, ACCOUNT_VERSION,
};
use super::types::{
    AccountOutcome, AccountRequest, AccountResponse, AccountResults, RequestHandle, UserInfo,
};
use portal_core::{RequestClose, PORTAL_PATH, RESPONSE_CANCELLED, RESPONSE_SUCCESS};

/// Pending request: raw message + resolved/in-flight user info to return on grant.
struct PendingRequest {
    raw: Rc<Message>,
    app_id: String,
    reason: Option<String>,
    user_info: Option<UserInfo>,
}

#[derive(State)]
pub struct AccountBackend {
    proxy: DbusProxy<SessionBus>,
    system_proxy: DbusProxy<SystemBus>,
    pending: HashMap<RequestHandle, PendingRequest>,
    find_user_calls: Pending<FindUserById, RequestHandle>,
    get_properties_calls: Pending<GetUserProperties, RequestHandle>,
}

impl AccountBackend {
    pub fn new(proxy: DbusProxy<SessionBus>, system_proxy: DbusProxy<SystemBus>) -> Self {
        Self {
            proxy,
            system_proxy,
            pending: HashMap::new(),
            find_user_calls: Pending::new(),
            get_properties_calls: Pending::new(),
        }
    }

    fn finish(&mut self, handle: &str, outcome: AccountOutcome) {
        let Some(entry) = self.pending.remove(handle) else {
            return;
        };
        let (response, results) = match outcome {
            AccountOutcome::Granted => {
                let user_info = entry.user_info.unwrap_or_else(|| UserInfo {
                    id: "user".to_string(),
                    name: "User".to_string(),
                    image: String::new(),
                });
                (
                    RESPONSE_SUCCESS,
                    AccountResults {
                        id: Some(user_info.id),
                        name: Some(user_info.name),
                        image: if user_info.image.is_empty() {
                            None
                        } else {
                            Some(user_info.image)
                        },
                    },
                )
            }
            AccountOutcome::Denied => (RESPONSE_CANCELLED, AccountResults::default()),
        };
        self.proxy.reply(&entry.raw, &(response, results));
    }

    fn fail(&mut self, handle: &str, error_name: &str, error_msg: &str) {
        if let Some(entry) = self.pending.remove(handle) {
            println!("[account-backend] Request failed: handle={handle} error={error_name}: {error_msg}");
            self.proxy.reply_error(&entry.raw, error_name, error_msg);
        }
    }

    fn trigger_consent_dialog(&mut self, handle: &str, user_info: UserInfo) -> Option<AccountRequest> {
        if let Some(req) = self.pending.get_mut(handle) {
            req.user_info = Some(user_info.clone());
            Some(AccountRequest::GetUserInformation {
                handle: handle.to_string(),
                app_id: req.app_id.clone(),
                reason: req.reason.clone(),
                user_info,
            })
        } else {
            None
        }
    }
}

pub fn account_backend_module<S>() -> impl RegisteredModule<AccountBackend, S> {
    Module::<AccountBackend, _, _>::new()
        // ── Handle UI decision ─────────────────────────────────────────────
        .on(|s: &mut AccountBackend, resp: &AccountResponse| {
            println!(
                "[account-backend] AccountResponse: handle={} outcome={:?}",
                resp.handle, resp.outcome
            );
            s.finish(&resp.handle, resp.outcome.clone());
        })
        .on(
            |s: &mut AccountBackend, ev: &DbusEvent<SessionBus>| -> Option<AccountRequest> {
                // ── GetUserInformation ─────────────────────────────────────────
                match IncomingCall::<GetUserInformation>::try_from(&ev.msg) {
                    Some(Ok(call)) => {
                        let (handle, app_id, _parent_window, options) = &call.args;

                        println!(
                            "[account-backend] GetUserInformation: app_id={app_id} handle={} reason={:?}",
                            handle.as_str(),
                            options.reason
                        );

                        let request_handle = handle.as_str().to_string();
                        s.pending.insert(
                            request_handle.clone(),
                            PendingRequest {
                                raw: call.raw().clone(),
                                app_id: app_id.clone(),
                                reason: options.reason.clone(),
                                user_info: None,
                            },
                        );

                        // Trigger AccountsService D-Bus async chain
                        let uid = get_current_uid();
                        s.find_user_calls.call(
                            &s.system_proxy,
                            &(uid as i64,),
                            request_handle,
                        );

                        return None; // UI spawn is deferred until D-Bus call resolves
                    }
                    Some(Err(e)) => {
                        eprintln!(
                            "[account-backend] GetUserInformation deserialization error: {e}"
                        );
                    }
                    None => {}
                }

                // ── Request.Close ──────────────────────────────────────────────
                if let Some(Ok(call)) = IncomingCall::<RequestClose>::try_from(&ev.msg) {
                    if let Some(handle) = &call.path {
                        call.respond(&s.proxy, &());
                        if s.pending.contains_key(handle) {
                            let handle = handle.clone();
                            s.finish(&handle, AccountOutcome::Denied);
                            return Some(AccountRequest::Close { handle });
                        }
                    }
                    return None;
                }

                // ── Properties: version ────────────────────────────────────────
                if fdo::route_properties(
                    &s.proxy,
                    &ev.msg,
                    ACCOUNT_IFACE,
                    &["version"],
                    |access| match access {
                        fdo::PropAccess::Get("version") => {
                            fdo::PropReply::Value(variant(ACCOUNT_VERSION))
                        }
                        fdo::PropAccess::Set("version", _) => fdo::PropReply::ReadOnly,
                        _ => fdo::PropReply::Unknown,
                    },
                ) {
                    return None;
                }

                // ── Fallback: unknown method on our interface ───────────────────
                if let DbusMessage::Call(m) = &ev.msg {
                    if m.header().path().is_some_and(|p| p.as_str() == PORTAL_PATH)
                        && m.header()
                            .interface()
                            .is_some_and(|i| i.as_str() == ACCOUNT_IFACE)
                    {
                        eprintln!(
                            "[account-backend] FALLBACK: unknown method={:?} on Account interface",
                            m.header().member()
                        );
                        s.proxy.reply_unknown_method(m);
                    }
                }

                None
            },
        )
        .on(
            |s: &mut AccountBackend, ev: &DbusEvent<SystemBus>| -> Option<AccountRequest> {
                // ── Handle FindUserById replies ─────────────────────────────────
                if let Some((handle, res)) = s.find_user_calls.resolve(&ev.msg) {
                    match res {
                        Ok(user_object_path) => {
                            println!(
                                "[account-backend] Found user path: {} for handle={handle}",
                                user_object_path.as_str()
                            );
                            s.get_properties_calls.call_at(
                                &s.system_proxy,
                                user_object_path.as_str(),
                                &("org.freedesktop.Accounts.User".to_string(),),
                                handle,
                            );
                        }
                        Err(e) => {
                            let error_msg = format!("AccountsService FindUserById failed: {e}");
                            eprintln!("[account-backend] {error_msg}");
                            s.fail(&handle, "org.freedesktop.portal.Error.Failed", &error_msg);
                        }
                    }
                    return None;
                }

                // ── Handle GetUserProperties replies ────────────────────────────
                if let Some((handle, res)) = s.get_properties_calls.resolve(&ev.msg) {
                    match res {
                        Ok(properties) => {
                            let id = properties
                                .get("UserName")
                                .cloned()
                                .and_then(|v| String::try_from(v).ok())
                                .unwrap_or_else(|| "user".to_string());
                            let name = properties
                                .get("RealName")
                                .cloned()
                                .and_then(|v| String::try_from(v).ok())
                                .unwrap_or_else(|| id.clone());
                            let image = properties
                                .get("IconFile")
                                .cloned()
                                .and_then(|v| String::try_from(v).ok())
                                .unwrap_or_default();

                            let mut image_uri = image.clone();
                            if !image_uri.is_empty() && !image_uri.starts_with("file://") && image_uri.starts_with('/') {
                                image_uri = format!("file://{}", image_uri);
                            }

                            let user_info = UserInfo {
                                id,
                                name,
                                image: image_uri,
                            };
                            return s.trigger_consent_dialog(&handle, user_info);
                        }
                        Err(e) => {
                            let error_msg = format!("AccountsService GetAll properties failed: {e}");
                            eprintln!("[account-backend] {error_msg}");
                            s.fail(&handle, "org.freedesktop.portal.Error.Failed", &error_msg);
                        }
                    }
                }

                None
            }
        )
}

/// Returns the current process UID (e.g., 1000 for user, 0 for root).
fn get_current_uid() -> i64 {
    unsafe { libc::getuid() as i64 }
}

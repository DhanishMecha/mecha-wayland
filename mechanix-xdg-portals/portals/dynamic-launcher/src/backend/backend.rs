use std::collections::HashMap;
use std::rc::Rc;

use app::{RegisteredModule, prelude::*};
use dbus::{DbusEvent, DbusMessage, DbusProxy, IncomingCall, SessionBus, fdo, variant};
use zbus::message::Message;
use zbus::zvariant::{OwnedValue, Value};

use super::interface::{
    DYNAMIC_LAUNCHER_IFACE, DYNAMIC_LAUNCHER_VERSION, PrepareInstall, RequestInstallToken,
};
use super::types::{
    DynamicLauncherOutcome, DynamicLauncherRequest, DynamicLauncherRequestInfo,
    DynamicLauncherResponse, RequestHandle,
};
use portal_core::{PORTAL_PATH, RESPONSE_CANCELLED, RESPONSE_SUCCESS, RequestClose};

struct PendingRequest {
    raw: Rc<Message>,
    name: String,
    icon_v: OwnedValue,
}

#[derive(State)]
pub struct DynamicLauncherBackend {
    proxy: DbusProxy<SessionBus>,
    pending: HashMap<RequestHandle, PendingRequest>,
}

impl DynamicLauncherBackend {
    pub fn new(proxy: DbusProxy<SessionBus>) -> Self {
        Self {
            proxy,
            pending: HashMap::new(),
        }
    }

    fn finish(&mut self, handle: &str, outcome: DynamicLauncherOutcome) {
        let Some(entry) = self.pending.remove(handle) else {
            return;
        };

        let (response, results) = match outcome {
            DynamicLauncherOutcome::Granted => {
                let mut res_map = HashMap::new();
                res_map.insert(
                    "name".to_string(),
                    OwnedValue::try_from(Value::Str(entry.name.into())).unwrap(),
                );
                res_map.insert("icon".to_string(), entry.icon_v);
                (RESPONSE_SUCCESS, res_map)
            }
            DynamicLauncherOutcome::Denied => (RESPONSE_CANCELLED, HashMap::new()),
        };

        self.proxy.reply(&entry.raw, &(response, results));
    }
}

pub fn dynamic_launcher_backend_module<S>() -> impl RegisteredModule<DynamicLauncherBackend, S> {
    Module::<DynamicLauncherBackend, _, _>::new()
        .on(|s: &mut DynamicLauncherBackend, resp: &DynamicLauncherResponse| {
            println!(
                "[dynamic-launcher-backend] Response for handle={}: {:?}",
                resp.handle, resp.outcome
            );
            s.finish(&resp.handle, resp.outcome.clone());
        })
        .on(
            |s: &mut DynamicLauncherBackend, ev: &DbusEvent<SessionBus>| -> Option<DynamicLauncherRequest> {
                // PrepareInstall
                if let Some(Ok(call)) = IncomingCall::<PrepareInstall>::try_from(&ev.msg) {
                    let (handle, app_id, _parent, name, icon_v, options) = &call.args;
                    let handle_str = handle.as_str().to_string();

                    println!(
                        "[dynamic-launcher-backend] PrepareInstall: app_id={app_id} handle={handle_str} name={name}"
                    );

                    s.pending.insert(
                        handle_str.clone(),
                        PendingRequest {
                            raw: call.raw().clone(),
                            name: name.clone(),
                            icon_v: icon_v.clone(),
                        },
                    );

                    let launcher_type = options
                        .get("launcher_type")
                        .and_then(|v| u32::try_from(v.clone()).ok())
                        .unwrap_or(1); // 1 = Application

                    let target = options
                        .get("target")
                        .and_then(|v| String::try_from(v.clone()).ok());

                    return Some(DynamicLauncherRequest::PrepareInstall(
                        DynamicLauncherRequestInfo {
                            handle: handle_str,
                            app_id: app_id.clone(),
                            name: name.clone(),
                            icon_v: icon_v.clone(),
                            launcher_type,
                            target,
                        },
                    ));
                }

                // RequestInstallToken
                if let Some(Ok(call)) = IncomingCall::<RequestInstallToken>::try_from(&ev.msg) {
                    let (app_id, _options) = &call.args;
                    println!(
                        "[dynamic-launcher-backend] RequestInstallToken: app_id={app_id}"
                    );
                    const ALLOWED_APPS: &[&str] = &["org.mechanix.software"];
                    if ALLOWED_APPS.contains(&app_id.as_str()) {
                        println!(
                            "[dynamic-launcher-backend] RequestInstallToken: app_id={app_id} is trusted. Granting token."
                        );
                        call.respond(&s.proxy, &(RESPONSE_SUCCESS,));
                    } else {
                        println!(
                            "[dynamic-launcher-backend] RequestInstallToken: app_id={app_id} is untrusted. Cancelling request."
                        );
                        call.respond(&s.proxy, &(RESPONSE_CANCELLED,));
                    }
                    return None;
                }

                // RequestClose
                if let Some(Ok(call)) = IncomingCall::<RequestClose>::try_from(&ev.msg) {
                    if let Some(handle) = &call.path {
                        call.respond(&s.proxy, &());
                        if s.pending.contains_key(handle) {
                            let handle_str = handle.clone();
                            s.finish(&handle_str, DynamicLauncherOutcome::Denied);
                            return Some(DynamicLauncherRequest::Close { handle: handle_str });
                        }
                    }
                    return None;
                }

                // Properties
                if fdo::route_properties(
                    &s.proxy,
                    &ev.msg,
                    DYNAMIC_LAUNCHER_IFACE,
                    &["version", "SupportedLauncherTypes"],
                    |access| match access {
                        fdo::PropAccess::Get("version") => {
                            fdo::PropReply::Value(variant(DYNAMIC_LAUNCHER_VERSION))
                        }
                        fdo::PropAccess::Get("SupportedLauncherTypes") => {
                            // Bitmask: 1 (Application) | 2 (Webapp) = 3
                            fdo::PropReply::Value(variant(3_u32))
                        }
                        fdo::PropAccess::Set("version", _) | fdo::PropAccess::Set("SupportedLauncherTypes", _) => {
                            fdo::PropReply::ReadOnly
                        }
                        _ => fdo::PropReply::Unknown,
                    },
                ) {
                    return None;
                }

                // Fallback
                if let DbusMessage::Call(m) = &ev.msg {
                    if m.header().path().is_some_and(|p| p.as_str() == PORTAL_PATH)
                        && m.header()
                            .interface()
                            .is_some_and(|i| i.as_str() == DYNAMIC_LAUNCHER_IFACE)
                    {
                        eprintln!(
                            "[dynamic-launcher-backend] FALLBACK: unknown method={:?} on DynamicLauncher interface",
                            m.header().member()
                        );
                        s.proxy.reply_unknown_method(m);
                    }
                }

                None
            },
        )
}

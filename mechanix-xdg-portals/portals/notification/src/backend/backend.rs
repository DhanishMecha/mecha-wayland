use app::{RegisteredModule, prelude::*};
use dbus::{DbusEvent, DbusMessage, DbusProxy, IncomingCall, SessionBus, fdo, variant};
use portal_core::PORTAL_PATH;
use std::collections::HashMap;

use super::interface::{
    ActionInvoked, AddNotification, NOTIFICATION_IFACE, NOTIFICATION_VERSION, RemoveNotification,
};
use super::types::{NotificationInfo, NotificationRequest, NotificationResponse};

#[derive(State)]
pub struct NotificationBackend {
    proxy: DbusProxy<SessionBus>,
    // Tracks active notifications by (app_id, notification_id)
    active_notifications: HashMap<(String, String), NotificationInfo>,
}

impl NotificationBackend {
    pub fn new(proxy: DbusProxy<SessionBus>) -> Self {
        Self {
            proxy,
            active_notifications: HashMap::new(),
        }
    }

    /// Helper to invoke an action on a notification (e.g. when clicked by user).
    /// Emits the `ActionInvoked` signal back to the client application.
    pub fn invoke_action(
        &self,
        app_id: &str,
        id: &str,
        action: &str,
        parameter: Vec<zbus::zvariant::OwnedValue>,
    ) {
        println!(
            "[notification-backend] Action invoked for app_id={}, id={}, action={}",
            app_id, id, action
        );
        self.proxy.emit::<ActionInvoked>(
            PORTAL_PATH,
            &(
                app_id.to_string(),
                id.to_string(),
                action.to_string(),
                parameter,
            ),
        );
    }
}

pub fn notification_backend_module<S>() -> impl RegisteredModule<NotificationBackend, S> {
    Module::<NotificationBackend, _, _>::new()
        .on(|s: &mut NotificationBackend, resp: &NotificationResponse| {
            s.invoke_action(&resp.app_id, &resp.id, &resp.action, resp.parameter.clone());
        })
        .on(
            |s: &mut NotificationBackend, ev: &DbusEvent<SessionBus>| -> Option<NotificationRequest> {
                // AddNotification
                if let Some(Ok(call)) = IncomingCall::<AddNotification>::try_from(&ev.msg) {
                    let (app_id, id, notification_map) = &call.args;
                    println!(
                        "[notification-backend] AddNotification: app_id={}, id={}",
                        app_id, id
                    );

                    let info = super::helpers::parse_notification(notification_map.clone());
                    println!(
                        "  -> Title: '{}', Body: '{}', Priority: '{}'",
                        info.title, info.body, info.priority
                    );
                    if let Some(ref markup) = info.markup_body {
                        println!("     Markup Body: '{}'", markup);
                    }
                    if let Some(ref cat) = info.category {
                        println!("     Category: '{}'", cat);
                    }
                    if !info.display_hints.is_empty() {
                        println!("     Display Hints: {:?}", info.display_hints);
                    }
                    if info.sound.is_some() {
                        println!("     Sound: (serialized payload present)");
                    }
                    for (idx, btn) in info.buttons.iter().enumerate() {
                        println!(
                            "     Button {}: label='{}', action='{}'",
                            idx, btn.label, btn.action
                        );
                    }

                    // Keep track of the notification details
                    let key = (app_id.clone(), id.clone());
                    println!("[notification-backend] key: {}, {}", key.0, key.1);
                    s.active_notifications.insert(key, info.clone());

                    call.respond(&s.proxy, &());
                    return Some(NotificationRequest::Add {
                        app_id: app_id.clone(),
                        id: id.clone(),
                        info,
                    });
                }

                // RemoveNotification
                if let Some(Ok(call)) = IncomingCall::<RemoveNotification>::try_from(&ev.msg) {
                    let (app_id, id) = &call.args;
                    println!(
                        "[notification-backend] RemoveNotification: app_id={}, id={}",
                        app_id, id
                    );

                    let key = (app_id.clone(), id.clone());
                    s.active_notifications.remove(&key);

                    call.respond(&s.proxy, &());
                    return Some(NotificationRequest::Remove {
                        app_id: app_id.clone(),
                        id: id.clone(),
                    });
                }

                // Properties: version and SupportedOptions properties
                if fdo::route_properties(
                    &s.proxy,
                    &ev.msg,
                    NOTIFICATION_IFACE,
                    &["version", "SupportedOptions"],
                    |access| match access {
                        fdo::PropAccess::Get("version") => {
                            fdo::PropReply::Value(variant(NOTIFICATION_VERSION))
                        }
                        fdo::PropAccess::Get("SupportedOptions") => {
                            // Advertise supported categories and button-purposes
                            let mut opts = HashMap::new();
                            let categories = vec![
                                "email".to_string(),
                                "im".to_string(),
                                "device".to_string(),
                                "presence".to_string(),
                            ];
                            let button_purposes = vec!["default".to_string(), "dismiss".to_string()];
                            opts.insert(
                                "category".to_string(),
                                zbus::zvariant::OwnedValue::try_from(zbus::zvariant::Value::from(
                                    categories,
                                ))
                                .unwrap(),
                            );
                            opts.insert(
                                "button-purpose".to_string(),
                                zbus::zvariant::OwnedValue::try_from(zbus::zvariant::Value::from(
                                    button_purposes,
                                ))
                                .unwrap(),
                            );
                            fdo::PropReply::Value(variant(opts))
                        }
                        fdo::PropAccess::Set("version", _)
                        | fdo::PropAccess::Set("SupportedOptions", _) => fdo::PropReply::ReadOnly,
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
                            .is_some_and(|i| i.as_str() == NOTIFICATION_IFACE)
                    {
                        eprintln!(
                            "[notification-backend] FALLBACK: unknown method on Notification interface"
                        );
                        s.proxy.reply_unknown_method(m);
                    }
                }

                None
            },
        )
}

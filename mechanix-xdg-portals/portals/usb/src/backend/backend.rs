use std::collections::HashMap;
use std::rc::Rc;

use app::{RegisteredModule, prelude::*};
use dbus::{DbusEvent, DbusMessage, DbusProxy, IncomingCall, SessionBus, fdo, variant};
use zbus::message::Message;
use zbus::zvariant::Value;

use super::interface::{AcquireDevices, USB_IFACE, USB_VERSION};
use super::types::{
    RequestHandle, UsbOutcome, UsbRequest, UsbRequestInfo, UsbResponse, UsbResultsDict,
};
use portal_core::{PORTAL_PATH, RESPONSE_CANCELLED, RESPONSE_SUCCESS, RequestClose};

// Backend state

#[derive(State)]
pub struct UsbBackend {
    proxy: DbusProxy<SessionBus>,
    pending: HashMap<RequestHandle, Rc<Message>>,
}

impl UsbBackend {
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

    fn finish(&mut self, handle: &str, outcome: UsbOutcome) {
        let Some(raw) = self.pending.remove(handle) else {
            return;
        };

        match outcome {
            UsbOutcome::Granted(allowed) => {
                println!("[usb-backend] Granted {} device(s)", allowed.len());
                for (dev_id, opts) in &allowed {
                    println!("  device_id={dev_id}  opts={opts:?}");
                }

                let devices: Vec<(String, HashMap<String, Value<'static>>)> = allowed
                    .into_iter()
                    .map(|(dev_id, opts)| {
                        let wrapped: HashMap<String, Value<'static>> = opts
                            .into_iter()
                            .map(|(k, v)| {
                                // OwnedValue → Value<'static>
                                let inner: Value<'static> = v.into();
                                // Wrap as a D-Bus Variant so the dict is a{sv}
                                (k, Value::Value(Box::new(inner)))
                            })
                            .collect();
                        (dev_id, wrapped)
                    })
                    .collect();

                let results = UsbResultsDict {
                    devices: Some(devices),
                };
                self.proxy.reply(&raw, &(RESPONSE_SUCCESS, results));
            }

            UsbOutcome::Denied => {
                self.proxy
                    .reply(&raw, &(RESPONSE_CANCELLED, UsbResultsDict::default()));
            }
        }
    }
}

pub fn usb_backend_module<S>() -> impl RegisteredModule<UsbBackend, S> {
    Module::<UsbBackend, _, _>::new()
        .on(|s: &mut UsbBackend, done: &UsbResponse| {
            s.finish(&done.handle, done.outcome.clone());
        })
        .on(
            |s: &mut UsbBackend, ev: &DbusEvent<SessionBus>| -> Option<UsbRequest> {
                // AcquireDevices()
                match IncomingCall::<AcquireDevices>::try_from(&ev.msg) {
                    Some(Ok(call)) => {
                        let (handle, parent_window, app_id, devices, _options) = &call.args;

                        println!(
                            "[usb-backend] AcquireDevices app={app_id} \
                             handle={} parent_window={parent_window}",
                            handle.as_str()
                        );

                        let handle_str = s.stash_pending(handle, call.raw());
                        return Some(UsbRequest::AcquireDevices(UsbRequestInfo {
                            handle: handle_str,
                            app_id: app_id.clone(),
                            devices: devices.clone(),
                        }));
                    }
                    Some(Err(e)) => {
                        eprintln!("[usb-backend] AcquireDevices deserialisation error: {e}");
                    }
                    None => {}
                }

                // Request.Close()
                if let Some(Ok(call)) = IncomingCall::<RequestClose>::try_from(&ev.msg) {
                    if let Some(handle) = &call.path {
                        call.respond(&s.proxy, &());
                        if s.pending.contains_key(handle) {
                            let h = handle.clone();
                            s.finish(&h, UsbOutcome::Denied);
                            return Some(UsbRequest::Close { handle: h });
                        }
                    }
                    return None;
                }

                // Properties: version
                if fdo::route_properties(&s.proxy, &ev.msg, USB_IFACE, &["version"], |access| {
                    match access {
                        fdo::PropAccess::Get("version") => {
                            fdo::PropReply::Value(variant(USB_VERSION))
                        }
                        fdo::PropAccess::Set("version", _) => fdo::PropReply::ReadOnly,
                        _ => fdo::PropReply::Unknown,
                    }
                }) {
                    return None;
                }

                // Fallback: unknown method on our interface
                if let DbusMessage::Call(m) = &ev.msg {
                    if m.header().path().is_some_and(|p| p.as_str() == PORTAL_PATH)
                        && m.header()
                            .interface()
                            .is_some_and(|i| i.as_str() == USB_IFACE)
                    {
                        eprintln!("[usb-backend] unknown method on Usb interface");
                        s.proxy.reply_unknown_method(m);
                    }
                }

                None
            },
        )
}

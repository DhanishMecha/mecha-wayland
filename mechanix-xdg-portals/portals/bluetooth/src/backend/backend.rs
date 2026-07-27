use std::rc::Rc;

use app::{RegisteredModule, prelude::*};
use dbus::{DbusEvent, DbusMessage, DbusProxy, IncomingCall, Pending, SystemBus};
use zbus::message::Message;
use zbus::zvariant::OwnedObjectPath;

use super::interface::{
    AGENT_PATH, AuthorizeService, BlueZAgent, Cancel, DisplayPasskey, DisplayPinCode,
    RegisterAgent, Release, RequestAuthorization, RequestConfirmation, RequestDefaultAgent,
    RequestPasskey, RequestPinCode,
};
use super::types::{AgentCapability, BluetoothOutcome, BluetoothRequest, BluetoothResponse};

pub const AGENT_CAPABILITY: AgentCapability = AgentCapability::DisplayYesNo;

#[derive(State)]
pub struct BluetoothBackend {
    proxy: DbusProxy<SystemBus>,
    register_agent: Pending<RegisterAgent>,
    request_default_agent: Pending<RequestDefaultAgent>,
    pending: Option<Rc<Message>>,
}

impl BluetoothBackend {
    pub fn new(proxy: DbusProxy<SystemBus>) -> Self {
        Self {
            proxy,
            register_agent: Pending::new(),
            request_default_agent: Pending::new(),
            pending: None,
        }
    }

    fn bootstrap(&mut self) {
        let path = OwnedObjectPath::try_from(AGENT_PATH).expect("valid agent path");
        self.register_agent
            .call(&self.proxy, &(path, AGENT_CAPABILITY.to_string()), ());
    }

    fn stash_pending(&mut self, raw: &Rc<Message>) {
        if let Some(old) = self.pending.take() {
            self.proxy.reply_error(
                &old,
                "org.bluez.Error.Cancelled",
                "superseded by new request",
            );
        }
        self.pending = Some(Rc::clone(raw));
    }

    fn finish_dialog(&mut self, outcome: BluetoothOutcome) {
        let Some(raw) = self.pending.take() else {
            return;
        };
        match outcome {
            BluetoothOutcome::Accepted => {
                self.proxy.reply(&raw, &());
            }
            BluetoothOutcome::PinCode(ref pin) => {
                self.proxy.reply(&raw, &pin.as_str());
            }
            BluetoothOutcome::Passkey(n) => {
                self.proxy.reply(&raw, &n);
            }
            BluetoothOutcome::Rejected => {
                self.proxy
                    .reply_error(&raw, "org.bluez.Error.Rejected", "rejected by user");
            }
            BluetoothOutcome::Dismissed => {
                self.proxy
                    .reply_error(&raw, "org.bluez.Error.Canceled", "dismissed by user");
            }
        }
    }

    fn cancel(&mut self) {
        if let Some(raw) = self.pending.take() {
            self.proxy
                .reply_error(&raw, "org.bluez.Error.Canceled", "cancelled by BlueZ");
        }
    }
}

pub fn bluetooth_module<S>() -> impl RegisteredModule<BluetoothBackend, S> {
    Module::<BluetoothBackend, _, _>::new()
        .on(|s: &mut BluetoothBackend, _: &app::Start| s.bootstrap())
        .on(|s: &mut BluetoothBackend, resp: &BluetoothResponse| {
            s.finish_dialog(resp.outcome.clone());
        })
        .on(
            |s: &mut BluetoothBackend, ev: &DbusEvent<SystemBus>| -> Option<BluetoothRequest> {
                // Reconnect/Disconnect handler for dbus
                match &ev.msg {
                    DbusMessage::Reconnected => {
                        println!("[bluetooth] system bus reconnected, re-registering agent");
                        s.bootstrap();
                        return None;
                    }
                    DbusMessage::Disconnected => {
                        s.pending = None;
                        s.register_agent.clear();
                        s.request_default_agent.clear();
                        println!("[bluetooth] system bus disconnected");
                        return None;
                    }
                    _ => {}
                }

                // RegisterAgent reply
                if let Some((_, res)) = s.register_agent.resolve(&ev.msg) {
                    match res {
                        Ok(()) => {
                            println!("[bluetooth] agent registered at {AGENT_PATH} (cap={AGENT_CAPABILITY}), requesting default");
                            let path =
                                OwnedObjectPath::try_from(AGENT_PATH).expect("valid agent path");
                            s.request_default_agent.call(&s.proxy, &(path,), ());
                        }
                        Err(e) => eprintln!("[bluetooth] RegisterAgent failed: {e}"),
                    }
                    return None;
                }

                // RequestDefaultAgent reply
                if let Some((_, res)) = s.request_default_agent.resolve(&ev.msg) {
                    match res {
                        Ok(()) => println!("[bluetooth] set as default agent"),
                        Err(e) => eprintln!("[bluetooth] RequestDefaultAgent failed: {e}"),
                    }
                    return None;
                }

                // Incoming Agent1 method calls from BlueZ

                // Release - BlueZ is unregistering us
                if let Some(Ok(call)) = IncomingCall::<Release>::try_from(&ev.msg) {
                    println!("[bluetooth] agent released by BlueZ");
                    call.respond(&s.proxy, &());
                    return None;
                }

                // RequestPinCode - stash and ask the UI for keyboard input.
                if let Some(Ok(call)) = IncomingCall::<RequestPinCode>::try_from(&ev.msg) {
                    let (device,) = &call.args;
                    println!("[bluetooth] RequestPinCode: device={}", device.as_str());
                    s.stash_pending(call.raw());
                    return Some(BluetoothRequest::RequestPinCode {
                        device: device.as_str().to_string(),
                    });
                }

                // DisplayPinCode - fire-and-forget; reply immediately.
                if let Some(Ok(call)) = IncomingCall::<DisplayPinCode>::try_from(&ev.msg) {
                    let (device, pincode) = &call.args;
                    println!("[bluetooth] DisplayPinCode: device={} pincode={}", device.as_str(), pincode);
                    call.respond(&s.proxy, &());
                    return Some(BluetoothRequest::DisplayPinCode {
                        device: device.as_str().to_string(),
                        pincode: pincode.clone(),
                    });
                }

                // RequestPasskey — stash and ask the UI for keyboard input.
                if let Some(Ok(call)) = IncomingCall::<RequestPasskey>::try_from(&ev.msg) {
                    let (device,) = &call.args;
                    println!("[bluetooth] RequestPasskey: device={}", device.as_str());
                    s.stash_pending(call.raw());
                    return Some(BluetoothRequest::RequestPasskey {
                        device: device.as_str().to_string(),
                    });
                }

                // DisplayPasskey — fire-and-forget; reply immediately.
                if let Some(Ok(call)) = IncomingCall::<DisplayPasskey>::try_from(&ev.msg) {
                    let (device, passkey, entered) = &call.args;
                    println!("[bluetooth] DisplayPasskey: device={} passkey={} entered={}", device.as_str(), passkey, entered);
                    call.respond(&s.proxy, &());
                    return Some(BluetoothRequest::DisplayPasskey {
                        device: device.as_str().to_string(),
                        passkey: *passkey,
                        entered: *entered,
                    });
                }

                // RequestConfirmation — stash and ask the UI.
                if let Some(Ok(call)) = IncomingCall::<RequestConfirmation>::try_from(&ev.msg) {
                    let (device, passkey) = &call.args;
                    println!("[bluetooth] RequestConfirmation: device={} passkey={}", device.as_str(), passkey);
                    s.stash_pending(call.raw());
                    return Some(BluetoothRequest::RequestConfirmation {
                        device: device.as_str().to_string(),
                        passkey: *passkey,
                    });
                }

                // RequestAuthorization — stash and ask the UI.
                if let Some(Ok(call)) = IncomingCall::<RequestAuthorization>::try_from(&ev.msg) {
                    let (device,) = &call.args;
                    println!("[bluetooth] RequestAuthorization: device={}", device.as_str());
                    s.stash_pending(call.raw());
                    return Some(BluetoothRequest::RequestAuthorization {
                        device: device.as_str().to_string(),
                    });
                }

                // AuthorizeService — stash and ask the UI.
                if let Some(Ok(call)) = IncomingCall::<AuthorizeService>::try_from(&ev.msg) {
                    let (device, uuid) = &call.args;
                    println!("[bluetooth] AuthorizeService: device={} uuid={}", device.as_str(), uuid);
                    s.stash_pending(call.raw());
                    return Some(BluetoothRequest::AuthorizeService {
                        device: device.as_str().to_string(),
                        uuid: uuid.clone(),
                    });
                }

                // Cancel — BlueZ is dismissing the current request.
                if let Some(Ok(call)) = IncomingCall::<Cancel>::try_from(&ev.msg) {
                    call.respond(&s.proxy, &());
                    s.cancel();
                    return Some(BluetoothRequest::Cancel);
                }

                // Standard interfaces
                if BlueZAgent::handle_standard(&s.proxy, AGENT_PATH, &ev.msg) {
                    return None;
                }

                // Unknown method call
                if let DbusMessage::Call(m) = &ev.msg {
                    if m.header().path().is_some_and(|p| p.as_str() == AGENT_PATH) {
                        eprintln!(
                            "[bluetooth] unknown method: interface={:?} member={:?}",
                            m.header().interface(),
                            m.header().member()
                        );
                        s.proxy.reply_unknown_method(m);
                    }
                }

                None
            },
        )
}

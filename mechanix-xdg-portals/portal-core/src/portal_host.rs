use app::{prelude::*, RegisteredModule};
use dbus::{fdo, DbusEvent, DbusMessage, DbusProxy, Pending, SessionBus};

use super::{PORTAL_NAME, PORTAL_PATH};

/// Owns the single `RequestName` for the whole portal service.
/// All individual portal backends (FileChooser, Access, …) listen on the same
/// session-bus event stream and handle only their own interface methods — they
/// do NOT call RequestName themselves.
#[derive(State)]
pub struct PortalHost {
    proxy: DbusProxy<SessionBus>,
    request_name: Pending<fdo::RequestName>,
    introspect_xml: String,
    pub owned: bool,
}

impl PortalHost {
    pub fn new(proxy: DbusProxy<SessionBus>, introspect_xml: String) -> Self {
        Self {
            proxy,
            request_name: Pending::new(),
            introspect_xml,
            owned: false,
        }
    }

    fn bootstrap(&mut self) {
        self.request_name.call(
            &self.proxy,
            &(PORTAL_NAME.to_string(), fdo::NAME_DO_NOT_QUEUE),
            (),
        );
    }
}

pub fn portal_host_module<S>() -> impl RegisteredModule<PortalHost, S>
where
    S: Lens<PortalHost> + 'static,
{
    Module::<PortalHost, _, _>::new()
        .on(|s: &mut PortalHost, _: &app::Start| s.bootstrap())
        .on(
            |s: &mut PortalHost, ev: &DbusEvent<SessionBus>| -> Option<()> {
                match &ev.msg {
                    DbusMessage::Reconnected => {
                        println!("PortalHost reconnected — re-registering {PORTAL_NAME}");
                        s.owned = false;
                        s.bootstrap();
                        return None;
                    }
                    DbusMessage::Disconnected => {
                        s.owned = false;
                        s.request_name.clear();
                        println!("PortalHost disconnected from D-Bus.");
                        return None;
                    }
                    _ => {}
                }

                if let Some((_, res)) = s.request_name.resolve(&ev.msg) {
                    match res {
                        Ok(code)
                            if code == fdo::REQUEST_NAME_PRIMARY_OWNER
                                || code == fdo::REQUEST_NAME_ALREADY_OWNER =>
                        {
                            s.owned = true;
                            println!("[portal-host] Serving {PORTAL_NAME} at {PORTAL_PATH}");
                        }
                        Ok(code) => {
                            eprintln!("[portal-host] Could not own {PORTAL_NAME} (code {code})")
                        }
                        Err(e) => eprintln!("[portal-host] RequestName failed: {e}"),
                    }
                    return None;
                }

                // Handle standard D-Bus interfaces centrally for PORTAL_PATH.
                // This includes Peer.Ping, Peer.GetMachineId, and Introspectable.Introspect.
                // We set has_properties to true because our interfaces expose a version property.
                if fdo::handle_standard(&s.proxy, PORTAL_PATH, &s.introspect_xml, true, &ev.msg) {
                    return None;
                }

                None
            },
        )
}

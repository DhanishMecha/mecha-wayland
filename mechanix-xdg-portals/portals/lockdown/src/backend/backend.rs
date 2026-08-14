use app::{RegisteredModule, prelude::*};
use dbus::{DbusEvent, DbusMessage, DbusProxy, SessionBus, fdo, variant};
use portal_core::PORTAL_PATH;

use super::interface::{LOCKDOWN_IFACE, LOCKDOWN_VERSION};

#[derive(State)]
pub struct LockdownBackend {
    proxy: DbusProxy<SessionBus>,
}

impl LockdownBackend {
    pub fn new(proxy: DbusProxy<SessionBus>) -> Self {
        Self { proxy }
    }
}

pub fn lockdown_backend_module<S>() -> impl RegisteredModule<LockdownBackend, S> {
    Module::<LockdownBackend, _, _>::new().on(
        |s: &mut LockdownBackend, ev: &DbusEvent<SessionBus>| -> Option<()> {
            let property_names = [
                "version",
                "disable-printing",
                "disable-save-to-disk",
                "disable-application-handlers",
                "disable-location",
                "disable-camera",
                "disable-microphone",
                "disable-sound-output",
            ];

            if fdo::route_properties(&s.proxy, &ev.msg, LOCKDOWN_IFACE, &property_names, |access| {
                match access {
                    fdo::PropAccess::Get("version") => {
                        fdo::PropReply::Value(variant(LOCKDOWN_VERSION))
                    }
                    fdo::PropAccess::Get("disable-printing") => {
                        let val = get_gsettings_bool("org.gnome.desktop.lockdown", "disable-printing", false);
                        fdo::PropReply::Value(variant(val))
                    }
                    fdo::PropAccess::Get("disable-save-to-disk") => {
                        let val = get_gsettings_bool("org.gnome.desktop.lockdown", "disable-save-to-disk", false);
                        fdo::PropReply::Value(variant(val))
                    }
                    fdo::PropAccess::Get("disable-application-handlers") => {
                        let val = get_gsettings_bool("org.gnome.desktop.lockdown", "disable-application-handlers", false);
                        fdo::PropReply::Value(variant(val))
                    }
                    fdo::PropAccess::Get("disable-location") => {
                        let enabled = get_gsettings_bool("org.gnome.system.location", "enabled", true);
                        fdo::PropReply::Value(variant(!enabled))
                    }
                    fdo::PropAccess::Get("disable-camera") => {
                        let val = get_gsettings_bool("org.gnome.desktop.privacy", "disable-camera", false);
                        fdo::PropReply::Value(variant(val))
                    }
                    fdo::PropAccess::Get("disable-microphone") => {
                        let val = get_gsettings_bool("org.gnome.desktop.privacy", "disable-microphone", false);
                        fdo::PropReply::Value(variant(val))
                    }
                    fdo::PropAccess::Get("disable-sound-output") => {
                        let val = get_gsettings_bool("org.gnome.desktop.privacy", "disable-sound-output", false);
                        fdo::PropReply::Value(variant(val))
                    }
                    fdo::PropAccess::Set(_, _) => fdo::PropReply::ReadOnly,
                    _ => fdo::PropReply::Unknown,
                }
            }) {
                return None;
            }

            // Fallback for unknown methods on this interface
            if let DbusMessage::Call(m) = &ev.msg {
                if m.header().path().is_some_and(|p| p.as_str() == PORTAL_PATH)
                    && m.header()
                        .interface()
                        .is_some_and(|i| i.as_str() == LOCKDOWN_IFACE)
                {
                    eprintln!("[lockdown-backend] FALLBACK: unknown method on Lockdown interface");
                    s.proxy.reply_unknown_method(m);
                }
            }

            None
        },
    )
}

fn get_gsettings_bool(schema: &str, key: &str, default_val: bool) -> bool {
    let output = std::process::Command::new("gsettings")
        .args(&["get", schema, key])
        .output();
    match output {
        Ok(out) => {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_lowercase();
            if s == "true" {
                true
            } else if s == "false" {
                false
            } else {
                default_val
            }
        }
        Err(_) => default_val,
    }
}


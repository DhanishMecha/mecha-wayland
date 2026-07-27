use app::{RegisteredModule, prelude::*};
use dbus::{DbusEvent, DbusMessage, DbusProxy, IncomingCall, SessionBus, fdo, variant};
use io_ring::{IoEvent, RingProxy};
use std::path::PathBuf;

use super::helpers::{
    DEFAULT_SETTINGS_TOML, default_settings, filter_namespaces, load_from_file, lookup,
};
use super::interface::{Read, ReadAll, SETTINGS_IFACE, SETTINGS_VERSION, SettingChanged};
use super::types::SettingsMap;
use super::watcher::InotifyWatcher;
use portal_core::PORTAL_PATH;

const ERR_INVALID_ARGUMENT: &str = "org.freedesktop.portal.Error.InvalidArgument";

#[derive(State)]
pub struct SettingsBackend {
    proxy: DbusProxy<SessionBus>,
    store: SettingsMap,
    config_path: PathBuf,
    watcher: Option<InotifyWatcher>,
}

impl SettingsBackend {
    pub fn new(proxy: DbusProxy<SessionBus>, ring: RingProxy) -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let config_dir = PathBuf::from(home).join(".config/mechanix");
        let config_path = config_dir.join("settings.toml");

        // Write default TOML if it doesn't exist
        if !config_path.exists() {
            if let Err(e) = std::fs::create_dir_all(&config_dir) {
                eprintln!(
                    "[settings-backend] Failed to create config dir {}: {e}",
                    config_dir.display()
                );
            } else if let Err(e) = std::fs::write(&config_path, DEFAULT_SETTINGS_TOML) {
                eprintln!("[settings-backend] Failed to write default settings config: {e}");
            } else {
                println!(
                    "[settings-backend] Created default config at {}",
                    config_path.display()
                );
            }
        }

        // Try to load settings from file
        let store = match load_from_file(&config_path) {
            Ok(s) => {
                println!(
                    "[settings-backend] Successfully loaded settings from {}",
                    config_path.display()
                );
                s
            }
            Err(e) => {
                eprintln!(
                    "[settings-backend] Error loading {}: {e}. Using defaults.",
                    config_path.display()
                );
                default_settings()
            }
        };

        let watcher = match InotifyWatcher::new(&config_dir, ring) {
            Ok(w) => Some(w),
            Err(e) => {
                eprintln!("[settings-backend] Failed to start inotify watcher: {e}");
                None
            }
        };

        Self {
            proxy,
            store,
            config_path,
            watcher,
        }
    }

    // Reloads the settings file and emits SettingChanged signals for any modified keys.
    pub fn reload_config(&mut self) {
        println!("[settings-backend] inotify event: settings.toml modified. Reloading...");
        match load_from_file(&self.config_path) {
            Ok(new_store) => {
                for (ns, new_keys) in &new_store {
                    let old_keys = self.store.get(ns);
                    for (key, new_val) in new_keys {
                        let val_changed = match old_keys.and_then(|ok| ok.get(key)) {
                            Some(old_val) => old_val != new_val,
                            None => true,
                        };
                        if val_changed {
                            println!("[settings-backend] Detected change: {ns}.{key}");
                            self.proxy.emit::<SettingChanged>(
                                PORTAL_PATH,
                                &(ns.clone(), key.clone(), new_val.clone()),
                            );
                        }
                    }
                }
                self.store = new_store;
            }
            Err(e) => {
                eprintln!("[settings-backend] Reload failed: {e}");
            }
        }
    }
}

pub fn settings_backend_module<S>() -> impl RegisteredModule<SettingsBackend, S> {
    Module::<SettingsBackend, _, _>::new()
        .on(|s: &mut SettingsBackend, io: &IoEvent| {
            let IoEvent::Completed { token, result } = *io;
            if let Some(watcher) = &mut s.watcher {
                if watcher.process_event(token, result) {
                    s.reload_config();
                }
            }
        })
        .on(
            |s: &mut SettingsBackend, ev: &DbusEvent<SessionBus>| -> Option<()> {
                // ReadAll
                if let Some(Ok(call)) = IncomingCall::<ReadAll>::try_from(&ev.msg) {
                    let (namespaces,) = &call.args;
                    println!(
                        "[settings-backend] ReadAll requested, {} namespace filter(s)",
                        namespaces.len()
                    );
                    let result = filter_namespaces(&s.store, namespaces);
                    call.respond(&s.proxy, &(result,));
                    return None;
                }

                // Read
                if let Some(Ok(call)) = IncomingCall::<Read>::try_from(&ev.msg) {
                    let (namespace, key) = &call.args;
                    println!("[settings-backend] Read: {namespace}.{key}");
                    match lookup(&s.store, namespace, key) {
                        Some(v) => {
                            call.respond(&s.proxy, &(v.clone(),));
                        }
                        None => {
                            call.error(
                                &s.proxy,
                                ERR_INVALID_ARGUMENT,
                                &format!("Namespace '{namespace}' or key '{key}' not found"),
                            );
                        }
                    }
                    return None;
                }

                // Properties
                if fdo::route_properties(
                    &s.proxy,
                    &ev.msg,
                    SETTINGS_IFACE,
                    &["version"],
                    |access| match access {
                        fdo::PropAccess::Get("version") => {
                            fdo::PropReply::Value(variant(SETTINGS_VERSION))
                        }
                        fdo::PropAccess::Set("version", _) => fdo::PropReply::ReadOnly,
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
                            .is_some_and(|i| i.as_str() == SETTINGS_IFACE)
                    {
                        eprintln!(
                            "[settings-backend] FALLBACK: unknown method on Settings interface"
                        );
                        s.proxy.reply_unknown_method(m);
                    }
                }

                None
            },
        )
}

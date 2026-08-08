use std::collections::HashMap;
use std::rc::Rc;

use app::{prelude::*, RegisteredModule};
use dbus::{DbusEvent, DbusMessage, DbusProxy, IncomingCall, SessionBus, fdo, variant};
use zbus::message::Message;

use super::interface::{
    SetWallpaperURI, WALLPAPER_IFACE, WALLPAPER_VERSION,
};
use super::types::{
    RequestHandle, WallpaperOutcome, WallpaperRequest, WallpaperResponse,
};
use portal_core::{PORTAL_PATH, RequestClose, RESPONSE_CANCELLED, RESPONSE_SUCCESS};

#[derive(State)]
pub struct WallpaperBackend {
    proxy: DbusProxy<SessionBus>,
    pending: HashMap<RequestHandle, (Rc<Message>, String)>,
}

impl WallpaperBackend {
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
        uri: String,
    ) -> RequestHandle {
        let handle_str = handle.as_str().to_string();
        self.pending.insert(handle_str.clone(), (Rc::clone(raw), uri));
        handle_str
    }

    fn finish(&mut self, handle: &str, outcome: WallpaperOutcome) {
        let Some((raw, uri)) = self.pending.remove(handle) else {
            return;
        };
        match outcome {
            WallpaperOutcome::Granted => {
                save_wallpaper_to_settings(&uri);
                self.proxy.reply(&raw, &(RESPONSE_SUCCESS,));
            }
            WallpaperOutcome::Denied => {
                self.proxy.reply(&raw, &(RESPONSE_CANCELLED,));
            }
        }
    }
}

fn save_wallpaper_to_settings(uri: &str) {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let config_path = std::path::PathBuf::from(home)
        .join(".config/mechanix/settings.toml");
    
    // Attempt to load settings
    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[wallpaper-backend] Failed to read settings file: {e}");
            return;
        }
    };
    
    let mut doc = match content.parse::<toml::Table>() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[wallpaper-backend] Failed to parse settings TOML: {e}");
            return;
        }
    };
    
    // Traverse and insert nested org.freedesktop.appearance
    let mut appearance_table = None;
    if let Some(org) = doc.entry("org").or_insert_with(|| toml::Value::Table(toml::Table::new())).as_table_mut() {
        if let Some(fd) = org.entry("freedesktop").or_insert_with(|| toml::Value::Table(toml::Table::new())).as_table_mut() {
            if let Some(app) = fd.entry("appearance").or_insert_with(|| toml::Value::Table(toml::Table::new())).as_table_mut() {
                appearance_table = Some(app);
            }
        }
    }
    
    if let Some(appearance) = appearance_table {
        appearance.insert("wallpaper".to_string(), toml::Value::String(uri.to_string()));
    }

    let updated = doc.to_string();
    if let Err(e) = std::fs::write(&config_path, updated) {
        eprintln!("[wallpaper-backend] Failed to write settings file: {e}");
    } else {
        println!("[wallpaper-backend] Successfully updated settings.toml with wallpaper={uri}");
    }
}

pub fn wallpaper_backend_module<S>() -> impl RegisteredModule<WallpaperBackend, S> {
    Module::<WallpaperBackend, _, _>::new()
        .on(|s: &mut WallpaperBackend, done: &WallpaperResponse| {
            s.finish(&done.handle, done.outcome.clone());
        })
        .on(
            |s: &mut WallpaperBackend, ev: &DbusEvent<SessionBus>| -> Option<WallpaperRequest> {
                // SetWallpaperURI()
                match IncomingCall::<SetWallpaperURI>::try_from(&ev.msg) {
                    Some(Ok(call)) => {
                        let (handle, app_id, _parent, uri, options) = &call.args;
                        let show_preview = options.show_preview.unwrap_or(true);
                        let set_on = options.set_on.clone().unwrap_or_else(|| "both".to_string());
                        println!(
                            "[wallpaper-backend] SetWallpaperURI requested by app={app_id} \
                             uri={uri} show_preview={show_preview} set_on={set_on}"
                        );
                        let handle_str = s.stash_pending(handle, call.raw(), uri.clone());
                        return Some(WallpaperRequest::SetWallpaper {
                            handle: handle_str,
                            app_id: app_id.clone(),
                            uri: uri.clone(),
                            show_preview,
                            set_on,
                        });
                    }
                    Some(Err(e)) => {
                        eprintln!("[wallpaper-backend] SetWallpaperURI deserialization error: {e}");
                    }
                    None => {}
                }

                // Request.Close()
                if let Some(Ok(call)) = IncomingCall::<RequestClose>::try_from(&ev.msg) {
                    if let Some(handle) = &call.path {
                        call.respond(&s.proxy, &());
                        if s.pending.contains_key(handle) {
                            let h = handle.clone();
                            s.finish(&h, WallpaperOutcome::Denied);
                            return Some(WallpaperRequest::Close { handle: h });
                        }
                    }
                    return None;
                }

                // Properties: version
                if fdo::route_properties(
                    &s.proxy,
                    &ev.msg,
                    WALLPAPER_IFACE,
                    &["version"],
                    |access| match access {
                        fdo::PropAccess::Get("version") => {
                            fdo::PropReply::Value(variant(WALLPAPER_VERSION))
                        }
                        fdo::PropAccess::Set("version", _) => fdo::PropReply::ReadOnly,
                        _ => fdo::PropReply::Unknown,
                    },
                ) {
                    return None;
                }

                // Fallback: unknown method on our interface
                if let DbusMessage::Call(m) = &ev.msg {
                    if m.header().path().is_some_and(|p| p.as_str() == PORTAL_PATH)
                        && m.header()
                            .interface()
                            .is_some_and(|i| i.as_str() == WALLPAPER_IFACE)
                    {
                        eprintln!(
                            "[wallpaper-backend] FALLBACK: unknown_method on Wallpaper interface"
                        );
                        s.proxy.reply_unknown_method(m);
                    }
                }

                None
            },
        )
}

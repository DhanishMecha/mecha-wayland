use app::{RegisteredModule, prelude::*};
use dbus::{DbusEvent, DbusMessage, DbusProxy, IncomingCall, SessionBus, fdo, variant};
use portal_core::PORTAL_PATH;
use std::collections::HashMap;
use std::io::Write;

use super::interface::{RetrieveSecret, SECRET_IFACE, SECRET_VERSION};

const ERR_OTHER: &str = "org.freedesktop.portal.Error.Failed";

// A hardcoded 64-byte master secret for testing/stub implementation
const HARDCODED_SECRET: [u8; 64] = [0x5au8; 64]; // 'Z' repeated 64 times

#[derive(State)]
pub struct SecretBackend {
    proxy: DbusProxy<SessionBus>,
}

impl SecretBackend {
    pub fn new(proxy: DbusProxy<SessionBus>) -> Self {
        Self { proxy }
    }
}

pub fn secret_backend_module<S>() -> impl RegisteredModule<SecretBackend, S> {
    Module::<SecretBackend, _, _>::new().on(
        |s: &mut SecretBackend, ev: &DbusEvent<SessionBus>| -> Option<()> {
            if let Some(Ok(call)) = IncomingCall::<RetrieveSecret>::try_from(&ev.msg) {
                let (_handle, app_id, fd, _options) = &call.args;
                println!("[secret-backend] RetrieveSecret requested for app_id={app_id}");

                // TODO: Implement a secret agent to store and retrieve secrets.
                // For now, we return a hardcoded response as a stub.
                let secret_bytes = &HARDCODED_SECRET[..];

                use std::os::fd::{AsRawFd, FromRawFd};
                let raw_fd = fd.as_raw_fd();
                let duped = unsafe { libc::dup(raw_fd) };
                if duped < 0 {
                    eprintln!("[secret-backend] Failed to dup file descriptor");
                    call.error(&s.proxy, ERR_OTHER, "Failed to duplicate file descriptor");
                    return None;
                }
                let std_fd = unsafe { std::os::fd::OwnedFd::from_raw_fd(duped) };
                let mut file = std::fs::File::from(std_fd);
                if let Err(e) = file.write_all(secret_bytes) {
                    eprintln!("[secret-backend] Failed to write secret to file descriptor: {e}");
                    call.error(
                        &s.proxy,
                        ERR_OTHER,
                        "Failed to write secret to file descriptor",
                    );
                    return None;
                }

                println!("[secret-backend] Successfully returned hardcoded secret for app_id={app_id}");
                call.respond(&s.proxy, &(0u32, HashMap::new()));
                return None;
            }

            // Properties
            if fdo::route_properties(&s.proxy, &ev.msg, SECRET_IFACE, &["version"], |access| {
                match access {
                    fdo::PropAccess::Get("version") => {
                        fdo::PropReply::Value(variant(SECRET_VERSION))
                    }
                    fdo::PropAccess::Set("version", _) => fdo::PropReply::ReadOnly,
                    _ => fdo::PropReply::Unknown,
                }
            }) {
                return None;
            }

            // Fallback
            if let DbusMessage::Call(m) = &ev.msg {
                if m.header().path().is_some_and(|p| p.as_str() == PORTAL_PATH)
                    && m.header()
                        .interface()
                        .is_some_and(|i| i.as_str() == SECRET_IFACE)
                {
                    eprintln!("[secret-backend] FALLBACK: unknown method on Secret interface");
                    s.proxy.reply_unknown_method(m);
                }
            }

            None
        },
    )
}

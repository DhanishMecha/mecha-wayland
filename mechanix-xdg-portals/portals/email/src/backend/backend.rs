use app::{RegisteredModule, prelude::*};
use dbus::{DbusEvent, DbusMessage, DbusProxy, IncomingCall, SessionBus, fdo, variant};
use portal_core::{PORTAL_PATH, RESPONSE_SUCCESS, RESPONSE_ENDED, RequestClose};
use std::collections::HashMap;
use zbus::zvariant::OwnedValue;

use super::interface::{ComposeEmail, EMAIL_IFACE, EMAIL_VERSION};
use super::helpers::{percent_encode, percent_decode, parse_email_data, to_file_uri};

#[derive(State)]
pub struct EmailBackend {
    proxy: DbusProxy<SessionBus>,
}

impl EmailBackend {
    pub fn new(proxy: DbusProxy<SessionBus>) -> Self {
        Self { proxy }
    }
}

pub fn email_backend_module<S>() -> impl RegisteredModule<EmailBackend, S> {
    Module::<EmailBackend, _, _>::new().on(
        |s: &mut EmailBackend, ev: &DbusEvent<SessionBus>| -> Option<()> {
            match &ev.msg {
                DbusMessage::Disconnected => return None,
                _ => {}
            }

            // ComposeEmail — launch the user's default email client.
            if let Some(Ok(call)) = IncomingCall::<ComposeEmail>::try_from(&ev.msg) {
                let (handle, app_id, _parent_window, options) = &call.args;
                let handle_str = handle.as_str();
                println!("options: {:?}", options);
                
                // Parse email data using our helper function
                let email_data = parse_email_data(options);

                println!(
                    "[email-backend] ComposeEmail: app_id={app_id} handle={handle_str}"
                );
                if let Some(addr) = &email_data.address {
                    println!("[email-backend]   address={addr}");
                }
                if !email_data.addresses.is_empty() {
                    println!("[email-backend]   addresses={}", email_data.addresses.join(", "));
                }
                if !email_data.cc.is_empty() {
                    println!("[email-backend]   cc={}", email_data.cc.join(", "));
                }
                if !email_data.bcc.is_empty() {
                    println!("[email-backend]   bcc={}", email_data.bcc.join(", "));
                }
                if let Some(subj) = &email_data.subject {
                    println!("[email-backend]   subject={subj}");
                }
                if let Some(b) = &email_data.body {
                    println!("[email-backend]   body={b}");
                }
                if !email_data.attachments.is_empty() {
                    println!("[email-backend]   attachments={}", email_data.attachments.join(", "));
                }

                // Prepare target addresses list
                let mut to_list: Vec<String> = email_data.addresses.clone();
                if let Some(addr) = &email_data.address {
                    to_list.push(addr.clone());
                }

                // 1. Try launching the client using xdg-email
                let mut cmd = std::process::Command::new("xdg-email");
                
                for cc_addr in &email_data.cc {
                    cmd.arg("--cc").arg(cc_addr);
                }
                for bcc_addr in &email_data.bcc {
                    cmd.arg("--bcc").arg(bcc_addr);
                }
                if let Some(subj) = &email_data.subject {
                    cmd.arg("--subject").arg(subj);
                }
                if let Some(b) = &email_data.body {
                    cmd.arg("--body").arg(b);
                }
                // TODO: attachment verification pending
                for attachment in &email_data.attachments {
                    let path = if attachment.starts_with("file://") {
                        percent_decode(&attachment["file://".len()..])
                    } else {
                        percent_decode(attachment)
                    };
                    cmd.arg("--attach").arg(path); // Local path for xdg-email
                }
                for to_addr in &to_list {
                    cmd.arg(to_addr);
                }
                let launch_result = cmd.spawn();
                match launch_result {
                    Ok(_) => {
                        println!("[email-backend] Launched email client via xdg-email.");
                        call.respond(&s.proxy, &(RESPONSE_SUCCESS, HashMap::<String, OwnedValue>::new()));
                    }
                    Err(e) => {
                        eprintln!("[email-backend] Failed to launch xdg-email: {e}. Falling back to mailto link via xdg-open...");
                        
                        // Fallback: Build a mailto: URI from the extracted options and open via xdg-open
                        let mut mailto = format!("mailto:{}", to_list.join(","));
                        let mut query_parts: Vec<String> = Vec::new();

                        if !email_data.cc.is_empty() {
                            query_parts.push(format!("cc={}", email_data.cc.join(",")));
                        }
                        if !email_data.bcc.is_empty() {
                            query_parts.push(format!("bcc={}", email_data.bcc.join(",")));
                        }
                        if let Some(subj) = &email_data.subject {
                            query_parts.push(format!("subject={}", percent_encode(subj)));
                        }
                        if let Some(b) = &email_data.body {
                            query_parts.push(format!("body={}", percent_encode(b)));
                        }
                        for attachment in &email_data.attachments {
                            let path = if attachment.starts_with("file://") {
                                percent_decode(&attachment["file://".len()..])
                            } else {
                                percent_decode(attachment)
                            };
                            let file_uri = to_file_uri(&path); // Format back to file:// URI
                            query_parts.push(format!("attachment={}", percent_encode(&file_uri)));
                        }

                        if !query_parts.is_empty() {
                            mailto.push('?');
                            mailto.push_str(&query_parts.join("&"));
                        }

                        println!("[email-backend]   mailto URI: {mailto}");

                        let fallback_result = std::process::Command::new("xdg-open").arg(&mailto).spawn();
                        match fallback_result {
                            Ok(_) => {
                                println!("[email-backend] Launched fallback email client via xdg-open.");
                                call.respond(&s.proxy, &(RESPONSE_SUCCESS, HashMap::<String, OwnedValue>::new()));
                            }
                            Err(err) => {
                                eprintln!("[email-backend] Failed to launch fallback xdg-open: {err}");
                                call.respond(&s.proxy, &(RESPONSE_ENDED, HashMap::<String, OwnedValue>::new()));
                            }
                        }
                    }
                }
                return None;
            }

            // Request.Close — unconditionally acknowledge (email is fire-and-forget).
            if let Some(Ok(call)) = IncomingCall::<RequestClose>::try_from(&ev.msg) {
                if let Some(handle) = &call.path {
                    println!(
                        "[email-backend] Request.Close received for handle={handle} (no-op for email)."
                    );
                    call.respond(&s.proxy, &());
                }
                return None;
            }

            // Properties: read-only `version`.
            if fdo::route_properties(
                &s.proxy,
                &ev.msg,
                EMAIL_IFACE,
                &["version"],
                |access| match access {
                    fdo::PropAccess::Get("version") => {
                        fdo::PropReply::Value(variant(EMAIL_VERSION))
                    }
                    fdo::PropAccess::Set("version", _) => fdo::PropReply::ReadOnly,
                    _ => fdo::PropReply::Unknown,
                },
            ) {
                return None;
            }

            // Fallback: unknown method on our interface.
            if let DbusMessage::Call(m) = &ev.msg {
                if m.header().path().is_some_and(|p| p.as_str() == PORTAL_PATH)
                    && m.header()
                        .interface()
                        .is_some_and(|i| i.as_str() == EMAIL_IFACE)
                {
                    eprintln!("[email-backend] FALLBACK: unknown method on Email interface");
                    s.proxy.reply_unknown_method(m);
                }
            }

            None
        },
    )
}



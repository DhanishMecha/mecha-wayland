use app::{RegisteredModule, prelude::*};
use dbus::{DbusEvent, DbusMessage, DbusProxy, IncomingCall, SessionBus, fdo, variant};
use portal_core::{PORTAL_PATH, RESPONSE_CANCELLED, RESPONSE_SUCCESS, RequestClose};
use std::collections::HashMap;
use std::rc::Rc;
use zbus::message::Message;
use zbus::zvariant::OwnedValue;

use super::interface::{PRINT_IFACE, PRINT_VERSION, PreparePrint, Print};
use super::types::{PrintOutcome, PrintRequest, PrintResponse, RequestHandle};

#[derive(Debug, Clone)]
pub struct CachedPrintJob {
    pub selected_printer: String,
    pub settings: HashMap<String, OwnedValue>,
    pub page_setup: HashMap<String, OwnedValue>,
}

#[derive(State)]
pub struct PrintBackend {
    proxy: DbusProxy<SessionBus>,
    pending: HashMap<RequestHandle, Rc<Message>>,
    jobs: HashMap<u32, CachedPrintJob>,
    token_counter: u32,
}

impl PrintBackend {
    pub fn new(proxy: DbusProxy<SessionBus>) -> Self {
        Self {
            proxy,
            pending: HashMap::new(),
            jobs: HashMap::new(),
            token_counter: 1,
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

    fn finish(&mut self, handle: &str, outcome: PrintOutcome) {
        let Some(raw) = self.pending.remove(handle) else {
            return;
        };
        match outcome {
            PrintOutcome::Granted {
                selected_printer,
                settings,
                page_setup,
            } => {
                let token = self.token_counter;
                self.token_counter += 1;

                self.jobs.insert(
                    token,
                    CachedPrintJob {
                        selected_printer: selected_printer.clone(),
                        settings: settings.clone(),
                        page_setup: page_setup.clone(),
                    },
                );

                let mut results = HashMap::new();
                results.insert("settings".to_string(), variant(settings));
                results.insert("page-setup".to_string(), variant(page_setup));
                results.insert("token".to_string(), variant(token));
                println!(
                    "[print-backend] PreparePrint GRANTED. Selected Printer: {}",
                    selected_printer
                );
                self.proxy.reply(&raw, &(RESPONSE_SUCCESS, results));
            }
            PrintOutcome::Denied => {
                println!("[print-backend] PreparePrint DENIED");
                self.proxy.reply(
                    &raw,
                    &(RESPONSE_CANCELLED, HashMap::<String, OwnedValue>::new()),
                );
            }
        }
    }
}

pub fn print_backend_module<S>() -> impl RegisteredModule<PrintBackend, S> {
    Module::<PrintBackend, _, _>::new()
        .on(|s: &mut PrintBackend, done: &PrintResponse| {
            s.finish(&done.handle, done.outcome.clone());
        })
        .on(
            |s: &mut PrintBackend, ev: &DbusEvent<SessionBus>| -> Option<PrintRequest> {
                // --- PreparePrint ---
                match IncomingCall::<PreparePrint>::try_from(&ev.msg) {
                    Some(Ok(call)) => {
                        let (handle, app_id, _parent_window, title, settings, page_setup, options) = &call.args;
                        println!("[print-backend] PreparePrint requested for app_id={app_id}, title={title}");

                        let handle_str = s.stash_pending(handle, call.raw());
                        return Some(PrintRequest::PreparePrint {
                            handle: handle_str,
                            app_id: app_id.clone(),
                            title: title.clone(),
                            settings: settings.clone(),
                            page_setup: page_setup.clone(),
                            options: options.clone(),
                        });
                    }
                    Some(Err(e)) => {
                        eprintln!("[print-backend] PreparePrint deserialization error: {e}");
                    }
                    None => {}
                }

                // --- Print ---
                match IncomingCall::<Print>::try_from(&ev.msg) {
                    Some(Ok(call)) => {
                        let (_handle, app_id, _parent_window, title, fd, options) = &call.args;
                        println!("[print-backend] Print requested for app_id={app_id}, title={title}");

                        let token = options.get("token").and_then(|v| {
                            use std::ops::Deref;
                            let val_ref = v.deref();
                            match val_ref {
                                zbus::zvariant::Value::U32(t) => Some(*t),
                                zbus::zvariant::Value::I32(t) => Some(*t as u32),
                                _ => None,
                            }
                        });

                        let Some(token_val) = token else {
                            eprintln!("[print-backend] Print failed: missing or invalid token option");
                            call.respond(
                                &s.proxy,
                                &(RESPONSE_CANCELLED, HashMap::<String, OwnedValue>::new()),
                            );
                            return None;
                        };

                        let Some(job) = s.jobs.remove(&token_val) else {
                            eprintln!("[print-backend] Print failed: job not found for token={token_val}");
                            call.respond(
                                &s.proxy,
                                &(RESPONSE_CANCELLED, HashMap::<String, OwnedValue>::new()),
                            );
                            return None;
                        };

                        println!(
                            "[print-backend] Found stashed job for token={token_val}. Printer={}",
                            job.selected_printer
                        );

                        // Read PDF from File Descriptor
                        use std::os::unix::io::AsRawFd;
                        use std::os::unix::io::FromRawFd;

                        let raw_fd = fd.as_raw_fd();
                        let dup_fd = unsafe { libc::dup(raw_fd) };
                        if dup_fd < 0 {
                            eprintln!("[print-backend] Print failed: unable to dup fd");
                            call.respond(
                                &s.proxy,
                                &(RESPONSE_CANCELLED, HashMap::<String, OwnedValue>::new()),
                            );
                            return None;
                        }

                        let mut input_file = unsafe { std::fs::File::from_raw_fd(dup_fd) };
                        
                        let output_path = job.settings.get("output-uri")
                            .and_then(|v| crate::helpers::format_value_to_string(v))
                            .map(|uri| {
                                if uri.starts_with("file://") {
                                    uri.trim_start_matches("file://").to_string()
                                } else {
                                    uri
                                }
                            })
                            .map(std::path::PathBuf::from);

                        let Some(dest_path) = output_path else {
                            eprintln!("[print-backend] Print failed: missing output-uri in settings");
                            call.respond(
                                &s.proxy,
                                &(RESPONSE_CANCELLED, HashMap::<String, OwnedValue>::new()),
                            );
                            return None;
                        };

                        if !dest_path.exists() {
                            eprintln!("[print-backend] Print failed: output file does not exist: {:?}", dest_path);
                            call.respond(
                                &s.proxy,
                                &(RESPONSE_CANCELLED, HashMap::<String, OwnedValue>::new()),
                            );
                            return None;
                        }

                        let mut dest_file = match std::fs::File::create(&dest_path) {
                            Ok(f) => f,
                            Err(e) => {
                                eprintln!("[print-backend] Print failed: unable to open output file: {e}");
                                call.respond(
                                    &s.proxy,
                                    &(RESPONSE_CANCELLED, HashMap::<String, OwnedValue>::new()),
                                );
                                return None;
                            }
                        };

                        if let Err(e) = std::io::copy(&mut input_file, &mut dest_file) {
                            eprintln!("[print-backend] Print failed: copy error: {e}");
                            call.respond(
                                &s.proxy,
                                &(RESPONSE_CANCELLED, HashMap::<String, OwnedValue>::new()),
                            );
                            return None;
                        }

                        // Print using helper
                        println!("[print-backend] Spooling print job to printer via helper...");
                        match crate::helpers::print_file(
                            &job.selected_printer,
                            title,
                            &dest_path,
                            &job.settings,
                            &job.page_setup,
                        ) {
                            Ok(()) => {
                                println!("[print-backend] Print job spooled successfully.");
                                call.respond(
                                    &s.proxy,
                                    &(RESPONSE_SUCCESS, HashMap::<String, OwnedValue>::new()),
                                );
                            }
                            Err(e) => {
                                eprintln!("[print-backend] Print helper failed: {e}");
                                call.respond(
                                    &s.proxy,
                                    &(RESPONSE_CANCELLED, HashMap::<String, OwnedValue>::new()),
                                );
                            }
                        }

                        return None;
                    }
                    Some(Err(e)) => {
                        eprintln!("[print-backend] Print deserialization error: {e}");
                    }
                    None => {}
                }

                // Request.Close()
                if let Some(Ok(call)) = IncomingCall::<RequestClose>::try_from(&ev.msg) {
                    if let Some(handle) = &call.path {
                        call.respond(&s.proxy, &());
                        if s.pending.contains_key(handle) {
                            let h = handle.clone();
                            s.finish(&h, PrintOutcome::Denied);
                            return Some(PrintRequest::Close { handle: h });
                        }
                    }
                    return None;
                }

                // Properties
                if fdo::route_properties(&s.proxy, &ev.msg, PRINT_IFACE, &["version"], |access| {
                    match access {
                        fdo::PropAccess::Get("version") => {
                            fdo::PropReply::Value(variant(PRINT_VERSION))
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
                            .is_some_and(|i| i.as_str() == PRINT_IFACE)
                    {
                        eprintln!("[print-backend] FALLBACK: unknown method on Print interface");
                        s.proxy.reply_unknown_method(m);
                    }
                }

                None
            },
        )
}


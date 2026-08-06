use std::collections::HashMap;

use app::{RegisteredModule, prelude::*};
use dbus::{
    DbusEvent, DbusMessage, DbusProxy, IncomingCall, Pending, SessionBus, SignalMatch, SystemBus,
};

use super::helpers::{get_session_id, process_start_time};
use super::interface::{
    AuthorityChanged, BeginAuthentication, CancelAuthentication,
    POLKIT_AGENT_IFACE, POLKIT_AGENT_PATH, RegisterAuthenticationAgent,
    UnregisterAuthenticationAgent,
};
use super::types::{
    ActiveAuth, AuthCancelled, AuthOutcome, AuthRequest, AuthResponse, CookieKey, Identity, Subject,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRegistrationState {
    Unregistered,
    Registering,
    Registered,
}

#[derive(State)]
pub struct PolkitAgentBackend {
    proxy: DbusProxy<SessionBus>,
    system_proxy: DbusProxy<SystemBus>,
    /// Active authentication sessions keyed by cookie.
    pending: HashMap<CookieKey, ActiveAuth>,
    /// Currently registered subject (unix-session or unix-process).
    current_subject: Option<Subject>,
    /// Current registration state with polkitd.
    registration_state: AgentRegistrationState,

    /// In-flight RegisterAuthenticationAgent calls tagged with the attempted Subject.
    register_calls: Pending<RegisterAuthenticationAgent, Subject>,
}

impl PolkitAgentBackend {
    pub fn new(proxy: DbusProxy<SessionBus>, system_proxy: DbusProxy<SystemBus>) -> Self {
        // Subscribe to the Authority "Changed" signal so we re-register if
        // polkitd restarts.
        system_proxy.subscribe::<AuthorityChanged>();

        let mut backend = Self {
            proxy,
            system_proxy,
            pending: HashMap::new(),
            current_subject: None,
            registration_state: AgentRegistrationState::Unregistered,
            register_calls: Pending::new(),
        };
        backend.register_agent();
        backend
    }

    /// Register this agent with polkitd.
    /// Attempts `unix-session` if session ID is available, otherwise uses `unix-process`.
    fn register_agent(&mut self) {
        let session_id = get_session_id();
        let subject = if !session_id.is_empty() {
            Subject::unix_session(&session_id)
        } else {
            Subject::unix_process(std::process::id(), process_start_time(std::process::id()))
        };

        self.register_subject(subject);
    }

    fn register_subject(&mut self, subject: Subject) {
        let locale = std::env::var("LANG").unwrap_or_else(|_| "en_US.UTF-8".to_string());
        let object_path = POLKIT_AGENT_PATH.to_string();

        println!(
            "[polkit-agent] Registering agent: subject_kind={}, details={:?}, locale={locale}, path={object_path}",
            subject.kind, subject.details,
        );

        self.current_subject = Some(subject.clone());
        self.registration_state = AgentRegistrationState::Registering;

        self.register_calls.call(
            &self.system_proxy,
            &(subject.to_dbus(), locale, object_path),
            subject,
        );
    }

    /// Unregister this agent from polkitd (called on shutdown / reconnect cleanup).
    fn unregister_agent(&mut self) {
        if let Some(subject) = self.current_subject.take() {
            let object_path = POLKIT_AGENT_PATH.to_string();
            println!("[polkit-agent] Unregistering agent with subject={}", subject.kind);
            self.system_proxy
                .call::<UnregisterAuthenticationAgent>(&(subject.to_dbus(), object_path));
        }
    }
}

impl Drop for PolkitAgentBackend {
    fn drop(&mut self) {
        println!("[polkit-agent] Backend dropping. Unregistering agent...");
        self.unregister_agent();
    }
}

pub fn polkit_agent_backend_module<S>() -> impl RegisteredModule<PolkitAgentBackend, S> {
    Module::<PolkitAgentBackend, _, _>::new()
        // -----------------------------------------------------------------
        // Handle the AuthResponse event from the dialog UI.
        // When the user submits or cancels the dialog, this fires.
        // -----------------------------------------------------------------
        .on(
            |s: &mut PolkitAgentBackend, resp: &AuthResponse| {
                let cookie = &resp.cookie;

                match &resp.outcome {
                    AuthOutcome::Cancelled => {
                        println!("[polkit-agent] Auth cancelled by user for cookie={cookie}");
                        if let Some(auth) = s.pending.remove(cookie) {
                            s.system_proxy.reply_error(
                                &auth.raw,
                                "org.freedesktop.PolicyKit1.Error.Cancelled",
                                "Authentication cancelled by user",
                            );
                        }
                    }
                    AuthOutcome::Authenticate { username, password } => {
                        println!(
                            "[polkit-agent] Auth submitted by user for cookie={cookie} user={username}"
                        );

                        // Perform PAM authentication via polkit-agent-helper-1
                        let helper_res = super::helpers::authenticate_with_pam(username, cookie, password);
                        if helper_res != super::helpers::HelperResult::Success {
                            let error_detail = match helper_res {
                                super::helpers::HelperResult::Failure(Some(msg)) => msg,
                                _ => "PAM authentication failed".to_string(),
                            };
                            eprintln!(
                                "[polkit-agent] PAM authentication failed for user={} cookie={}: {}",
                                username, cookie, error_detail
                            );
                            if let Some(auth) = s.pending.remove(cookie) {
                                s.system_proxy.reply_error(
                                    &auth.raw,
                                    "org.freedesktop.PolicyKit1.Error.Failed",
                                    &error_detail,
                                );
                            }
                            return;
                        }

                        println!("[polkit-agent] PAM authentication succeeded for user={username}");

                        if let Some(auth) = s.pending.remove(cookie) {
                            s.system_proxy.reply(&auth.raw, &());
                        }
                    }
                }
            },
        )
        // -----------------------------------------------------------------
        // System bus: incoming calls from polkitd
        // -----------------------------------------------------------------
        .on(
            |s: &mut PolkitAgentBackend, ev: &DbusEvent<SystemBus>| -> frunk::HCons<Option<AuthRequest>, frunk::HCons<Option<AuthCancelled>, frunk::HNil>> {
                match &ev.msg {
                    DbusMessage::Disconnected => return frunk::hlist![None, None],
                    _ => {}
                }

                // BeginAuthentication — polkitd wants the user to authenticate.
                if let Some(Ok(call)) = IncomingCall::<BeginAuthentication>::try_from(&ev.msg) {
                    let (action_id, message, icon_name, details, cookie, raw_identities) =
                        &call.args;

                    let identities: Vec<Identity> = raw_identities
                        .iter()
                        .cloned()
                        .map(Identity::from)
                        .collect();

                    println!(
                        "[polkit-agent] BeginAuthentication: action={action_id} cookie={cookie}"
                    );

                    let auth = ActiveAuth {
                        action_id: action_id.clone(),
                        message: message.clone(),
                        icon_name: icon_name.clone(),
                        details: details.clone(),
                        cookie: cookie.clone(),
                        identities: identities.clone(),
                        raw: call.raw().clone(),
                    };

                    // Stash the active auth. We do NOT reply here — we reply
                    // only after successful authentication or on cancel.
                    s.pending.insert(cookie.clone(), auth);

                    // Emit AuthRequest event to wake up the dialog UI module.
                    return frunk::hlist![
                        Some(AuthRequest {
                            cookie: cookie.clone(),
                            action_id: action_id.clone(),
                            message: message.clone(),
                            icon_name: icon_name.clone(),
                            details: details.clone(),
                            identities,
                        }),
                        None
                    ];
                }

                // CancelAuthentication — polkitd wants us to cancel an active auth.
                if let Some(Ok(call)) = IncomingCall::<CancelAuthentication>::try_from(&ev.msg) {
                    let (cookie,) = &call.args;
                    println!("[polkit-agent] CancelAuthentication: cookie={cookie}");

                    call.respond(&s.system_proxy, &());
                    if let Some(auth) = s.pending.remove(cookie) {
                        s.system_proxy.reply_error(
                            &auth.raw,
                            "org.freedesktop.PolicyKit1.Error.Cancelled",
                            "Authentication cancelled by polkitd",
                        );
                    }

                    // Tell the UI to close the dialog for this cookie.
                    return frunk::hlist![
                        None,
                        Some(AuthCancelled {
                            cookie: cookie.clone(),
                        })
                    ];
                }

                // Fallback: unknown method on our interface.
                if let DbusMessage::Call(m) = &ev.msg {
                    if m.header()
                        .path()
                        .is_some_and(|p| p.as_str() == POLKIT_AGENT_PATH)
                        && m.header()
                            .interface()
                            .is_some_and(|i| i.as_str() == POLKIT_AGENT_IFACE)
                    {
                        eprintln!(
                            "[polkit-agent] FALLBACK: unknown method={:?}",
                            m.header().member()
                        );
                        s.system_proxy.reply_unknown_method(m);
                    }
                }

                frunk::hlist![None, None]
            },
        )
        // -----------------------------------------------------------------
        // System bus: resolve Register / Response2 replies and Authority signals
        // -----------------------------------------------------------------
        .on(
            |s: &mut PolkitAgentBackend, ev: &DbusEvent<SystemBus>| -> Many<Vec<AuthCancelled>> {
                match &ev.msg {
                    DbusMessage::Disconnected => {
                        eprintln!("[polkit-agent] System bus disconnected. Clearing state.");
                        s.registration_state = AgentRegistrationState::Unregistered;
                        s.register_calls.clear();

                        // Clear pending auths and emit AuthCancelled for each so the UI drops its dialogs
                        let cancelled_events: Vec<AuthCancelled> = s.pending.drain()
                            .map(|(cookie, _)| AuthCancelled { cookie })
                            .collect();
                        
                        return Many(cancelled_events);
                    }
                    DbusMessage::Reconnected => {
                        println!("[polkit-agent] System bus reconnected. Re-registering agent.");
                        s.registration_state = AgentRegistrationState::Unregistered;
                        s.system_proxy.subscribe::<AuthorityChanged>();
                        s.register_agent();
                        return Many(Vec::new());
                    }
                    _ => {}
                }

                // Resolve RegisterAuthenticationAgent reply.
                if let Some((subject, res)) = s.register_calls.resolve(&ev.msg) {
                    match res {
                        Ok(()) => {
                            println!(
                                "[polkit-agent] Successfully registered agent with polkitd! \
                                 Kind: {}, Details: {:?}, Object Path: {}, Locale: {}",
                                subject.kind,
                                subject.details,
                                POLKIT_AGENT_PATH,
                                std::env::var("LANG").unwrap_or_else(|_| "en_US.UTF-8".to_string())
                            );
                            s.current_subject = Some(subject);
                            s.registration_state = AgentRegistrationState::Registered;
                        }
                        Err(e) => {
                            eprintln!(
                                "[polkit-agent] Registration failed for kind={}: {e}",
                                subject.kind
                            );
                            // If unix-session failed, retry with unix-process fallback
                            if subject.kind == "unix-session" {
                                println!(
                                    "[polkit-agent] Retrying registration using unix-process fallback (pid={})...",
                                    std::process::id()
                                );
                                s.register_subject(Subject::unix_process(
                                    std::process::id(),
                                    process_start_time(std::process::id()),
                                ));
                            } else {
                                s.registration_state = AgentRegistrationState::Unregistered;
                            }
                        }
                    }
                    return Many(Vec::new());
                }

                // Handle Authority "Changed" signal — re-register ONLY if polkitd restarted
                // and our agent is unregistered.
                if let Some(Ok(_)) = SignalMatch::<AuthorityChanged>::try_from(&ev.msg) {
                    if s.registration_state == AgentRegistrationState::Unregistered {
                        println!("[polkit-agent] Authority Changed signal received and agent is unregistered. Re-registering.");
                        s.register_agent();
                    }
                    return Many(Vec::new());
                }

                Many(Vec::new())
            },
        )
}


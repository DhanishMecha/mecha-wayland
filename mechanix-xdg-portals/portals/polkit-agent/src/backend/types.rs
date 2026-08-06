use std::collections::HashMap;
use std::rc::Rc;
use zbus::Message;
use zeroize::Zeroizing;

use app::Event;
use zbus::zvariant::{OwnedValue, Value};

pub type RequestHandle = String;

/// subject_kind: "unix-session" | "unix-process"
/// subject_details: e.g. {"session-id": <s "c1">} or {"pid": <u 1234>, "start-time": <t 0>}
#[derive(Debug, Clone)]
pub struct Subject {
    pub kind: String,
    pub details: HashMap<String, OwnedValue>,
}

impl Subject {
    /// Construct a unix-session subject (used for agent registration).
    pub fn unix_session(session_id: impl Into<String>) -> Self {
        let mut details = HashMap::new();
        details.insert(
            "session-id".to_string(),
            OwnedValue::try_from(Value::Str(session_id.into().into())).unwrap(),
        );
        Self {
            kind: "unix-session".to_string(),
            details,
        }
    }

    /// Construct a unix-process subject (used as fallback when process session differs).
    pub fn unix_process(pid: u32, start_time: u64) -> Self {
        let mut details = HashMap::new();
        details.insert(
            "pid".to_string(),
            OwnedValue::try_from(Value::U32(pid)).unwrap(),
        );
        details.insert(
            "start-time".to_string(),
            OwnedValue::try_from(Value::U64(start_time)).unwrap(),
        );
        Self {
            kind: "unix-process".to_string(),
            details,
        }
    }

    /// Convert to the D-Bus wire tuple `(String, HashMap<String, OwnedValue>)`.
    pub fn to_dbus(&self) -> (String, HashMap<String, OwnedValue>) {
        (self.kind.clone(), self.details.clone())
    }
}

/// A D-Bus Identity struct as per the polkit spec.
/// identity_kind: "unix-user" | "unix-group"
/// identity_details: e.g. {"uid": <u 1000>}
#[derive(Debug, Clone)]
pub struct Identity {
    pub kind: String,
    pub details: HashMap<String, OwnedValue>,
}

impl Identity {
    /// Extract uid from a unix-user identity, if present.
    pub fn uid(&self) -> Option<u32> {
        if self.kind != "unix-user" {
            return None;
        }
        self.details
            .get("uid")
            .and_then(|v| u32::try_from(v.clone()).ok())
    }
}

impl From<(String, HashMap<String, OwnedValue>)> for Identity {
    fn from((kind, details): (String, HashMap<String, OwnedValue>)) -> Self {
        Self { kind, details }
    }
}

/// An active authentication request, tracked by cookie.
#[derive(Debug, Clone)]
pub struct ActiveAuth {
    /// The polkit action being authenticated.
    pub action_id: String,
    /// Message to display to the user.
    pub message: String,
    /// Themed icon name (may be empty).
    pub icon_name: String,
    /// Key/value details from polkitd (e.g. polkit.caller-pid).
    pub details: HashMap<String, String>,
    /// The cookie identifying this request.
    pub cookie: String,
    /// The list of identities the user can authenticate as.
    pub identities: Vec<Identity>,
    /// Raw D-Bus message for the BeginAuthentication call.
    pub raw: Rc<Message>,
}

pub type CookieKey = String;

// ---------------------------------------------------------------------------
// Events exchanged between backend and dialog UI
// ---------------------------------------------------------------------------

/// Emitted by backend → dialog: "show an auth dialog for this request".
#[derive(Debug, Clone)]
pub struct AuthRequest {
    pub cookie: String,
    pub action_id: String,
    pub message: String,
    pub icon_name: String,
    pub details: HashMap<String, String>,
    pub identities: Vec<Identity>,
}
impl Event for AuthRequest {}

/// Emitted by dialog → backend: "user submitted / cancelled this auth".
#[derive(Debug, Clone)]
pub struct AuthResponse {
    pub cookie: String,
    pub outcome: AuthOutcome,
}
impl Event for AuthResponse {}

/// Emitted by backend → dialog: "cancel / close this auth dialog".
#[derive(Debug, Clone)]
pub struct AuthCancelled {
    pub cookie: String,
}
impl Event for AuthCancelled {}

#[derive(Debug, Clone)]
pub enum AuthOutcome {
    /// User provided credentials — contains the username and password.
    Authenticate { username: String, password: Zeroizing<String> },
    /// User dismissed the dialog.
    Cancelled,
}

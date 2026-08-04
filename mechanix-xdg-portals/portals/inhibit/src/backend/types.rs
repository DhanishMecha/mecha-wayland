use serde::{Deserialize, Serialize};

/// A unique identifier for a pending Request handle.
pub type RequestHandle = String;

/// A unique identifier for a monitoring Session handle.
pub type SessionHandle = String;

/// Parsed options from the Inhibit vardict.
#[derive(Debug, Clone, Default)]
pub struct InhibitOptions {
    /// Human-readable reason for the inhibition (shown in session manager UI).
    pub reason: Option<String>,
}

/// An active inhibition entry tracked by the backend.
#[derive(Debug)]
pub struct InhibitEntry {
    pub handle: RequestHandle,
    pub app_id: String,
    pub flags: u32,
    pub reason: Option<String>,
    pub fd: Option<zbus::zvariant::OwnedFd>,
}

/// An active monitoring session created via CreateMonitor.
/// While alive, it receives StateChanged signals.
#[derive(Debug, Clone)]
pub struct MonitorSession {
    pub session_handle: SessionHandle,
    pub app_id: String,
}

/// Session state values used in the `session-state` key of StateChanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)]
pub enum SessionState {
    /// Session is running normally.
    Running = 1,
    /// A logout/shutdown query is in progress — apps have ~1 second to respond
    /// via QueryEndResponse.
    QueryEnd = 2,
    /// Session is ending (no further response expected).
    Ending = 3,
}

impl SessionState {
    pub fn as_u32(self) -> u32 {
        self as u32
    }
}

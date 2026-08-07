/// A unique identifier for a clipboard session, derived from the session object path.
pub type SessionHandle = String;

/// An active clipboard session registered via RequestClipboard.
/// Clipboard sessions attach to existing portal sessions (e.g. RemoteDesktop).
#[derive(Debug, Clone)]
pub struct ClipboardSession {
    /// The session object path used as the key for this entry.
    pub session_handle: SessionHandle,

    /// MIME types currently advertised by this session via SetSelection.
    /// Empty until SetSelection is called by the owning session.
    pub mime_types: Vec<String>,

    /// Whether this session currently owns the clipboard selection.
    pub is_owner: bool,

    /// Monotonically increasing serial used to pair SelectionTransfer signals
    /// with WriteSelection replies.
    pub next_serial: u32,
}

impl ClipboardSession {
    pub fn new(session_handle: SessionHandle) -> Self {
        Self {
            session_handle,
            mime_types: Vec::new(),
            is_owner: false,
            next_serial: 0,
        }
    }

    /// Increment and return the next transfer serial.
    pub fn alloc_serial(&mut self) -> u32 {
        let s = self.next_serial;
        self.next_serial = self.next_serial.wrapping_add(1);
        s
    }
}

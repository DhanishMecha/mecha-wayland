use app::Event;
use zbus::zvariant::{DeserializeDict, SerializeDict, Type};

pub type RequestHandle = String;

/// Options for the Screenshot method.
#[derive(DeserializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct ScreenshotOptions {
    pub modal: Option<bool>,
    pub interactive: Option<bool>,
    pub permission_store_checked: Option<bool>,
    /// The screenshot target (1=Screen, 2=Window, 4=Area, 8=Active Window).
    pub target: Option<u32>,
}

#[derive(SerializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct ScreenshotResults {
    /// URI of the screenshot file that was saved (e.g. file:///tmp/…).
    pub uri: Option<String>,
}

#[derive(DeserializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct PickColorOptions {}

#[derive(SerializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct PickColorResults {
    pub color: Option<(f64, f64, f64)>,
}

/// Tracks which portal method a pending request handle belongs to.
/// reply type (ScreenshotResults vs PickColorResults) to send.
#[derive(Debug, Clone, Copy)]
pub enum RequestKind {
    Screenshot,
    PickColor,
}

/// Request sent from the backend to the dialog layer.
#[derive(Debug)]
pub enum ScreenshotRequest {
    /// User-visible confirmation required before taking a screenshot.
    Screenshot {
        handle: RequestHandle,
        app_id: String,
        /// If true the caller asked for an interactive mode (region/window pick).
        /// We still show the same confirmation dialog; the distinction is noted
        /// in the body text and carried to the response for future use.
        interactive: bool,
        target: Option<u32>,
    },
    /// User-visible confirmation required before picking a colour.
    PickColor {
        handle: RequestHandle,
        app_id: String,
    },
    /// The caller cancelled the request via Request.Close.
    Close { handle: RequestHandle },
}
impl Event for ScreenshotRequest {}

/// Response sent back from the dialog layer to the backend.
#[derive(Debug)]
pub struct ScreenshotResponse {
    pub handle: RequestHandle,
    pub outcome: ScreenshotOutcome,
}
impl Event for ScreenshotResponse {}

/// Whether the user allowed or denied the portal request.
#[derive(Debug, Clone)]
pub enum ScreenshotOutcome {
    /// User allowed; for Screenshot the backend will produce a URI.
    Granted,
    Denied,
}

use app::Event;
use zbus::zvariant::{DeserializeDict, SerializeDict, Type};

pub type RequestHandle = String;

#[derive(DeserializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct AccessOptions {
    /// Label for the "deny" button.
    pub deny_label: Option<String>,
    /// Label for the "grant" button.
    pub grant_label: Option<String>,
    /// Whether to show the dialog as modal.
    pub modal: Option<bool>,
    /// Icon name to display.
    pub icon: Option<String>,
    /// Additional choices
    pub choices: Option<Vec<(String, String, Vec<(String, String)>, String)>>,
}

/// Results returned to the caller after AccessDialog.
#[derive(SerializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct AccessResults {
    pub choices: Option<Vec<(String, String)>>,
}

#[derive(Debug)]
pub enum AccessRequest {
    AccessDialog {
        handle: RequestHandle,
        app_id: String,
        title: String,
        subtitle: String,
        body: String,
        options: AccessOptions,
    },
    Close {
        handle: RequestHandle,
    },
}
impl Event for AccessRequest {}

#[derive(Debug)]
pub struct AccessResponse {
    pub handle: RequestHandle,
    pub outcome: AccessOutcome,
}
impl Event for AccessResponse {}

#[derive(Debug, Clone)]
pub enum AccessOutcome {
    Granted,
    Denied,
}

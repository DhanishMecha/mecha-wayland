use app::Event;
use zbus::zvariant::{DeserializeDict, SerializeDict, Type};

pub type RequestHandle = String;

/// Options passed to ChooseApplication.
#[derive(DeserializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct AppChooserOptions {
    pub last_choice: Option<String>,
    pub modal: Option<bool>,
    pub content_type: Option<String>,
    pub activation_token: Option<String>,
    pub uri: Option<String>,
    pub filename: Option<String>,
}

/// Results returned after ChooseApplication.
#[derive(SerializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct AppChooserResults {
    pub choice: Option<String>,
    /// An activation token for the chosen application.
    pub activation_token: Option<String>,
}

/// Emitted by the D-Bus backend; consumed by the AppChooser UI.
#[derive(Debug)]
pub enum AppChooserRequest {
    ChooseApplication {
        handle: RequestHandle,
        app_id: String,
        choices: Vec<String>,
        last_choice: Option<String>,
        content_type: Option<String>,
        uri: Option<String>,
        filename: Option<String>,
    },
    UpdateChoices {
        handle: RequestHandle,
        choices: Vec<String>,
    },
    Close {
        handle: RequestHandle,
    },
}
impl Event for AppChooserRequest {}

/// Emitted by the AppChooser UI when user confirms/cancels; consumed by backend to reply.
#[derive(Debug)]
pub struct AppChooserResponse {
    pub handle: RequestHandle,
    pub outcome: AppChooserOutcome,
}
impl Event for AppChooserResponse {}

#[derive(Debug, Clone)]
pub enum AppChooserOutcome {
    Chosen(String),
    Cancelled,
}

use app::Event;

pub type RequestHandle = String;

#[derive(Debug, Clone)]
pub enum GlobalShortcutsRequest {
    BindDialog {
        handle: RequestHandle,
        app_id: String,
        shortcuts: Vec<(String, String)>, // includes preferred_trigger and description
    },
    ConfigureDialog {
        handle: RequestHandle,
        app_id: String,
        shortcuts: Vec<(String, String)>,
    },
    Close {
        handle: RequestHandle,
    },
}
impl Event for GlobalShortcutsRequest {}

#[derive(Debug, Clone)]
pub struct GlobalShortcutsResponse {
    pub handle: RequestHandle,
    pub outcome: GlobalShortcutsOutcome,
}
impl Event for GlobalShortcutsResponse {}

#[derive(Debug, Clone)]
pub enum GlobalShortcutsOutcome {
    Granted,
    Denied,
}

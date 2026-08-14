use app::Event;
use std::collections::HashMap;
use zbus::zvariant::OwnedValue;

pub type RequestHandle = String;

#[derive(Debug, Clone)]
pub enum PrintOutcome {
    Granted {
        selected_printer: String,
        settings: HashMap<String, OwnedValue>,
        page_setup: HashMap<String, OwnedValue>,
    },
    Denied,
}

#[derive(Debug)]
pub enum PrintRequest {
    PreparePrint {
        handle: RequestHandle,
        app_id: String,
        title: String,
        settings: HashMap<String, OwnedValue>,
        page_setup: HashMap<String, OwnedValue>,
        options: HashMap<String, OwnedValue>,
    },
    Close {
        handle: RequestHandle,
    },
}
impl Event for PrintRequest {}

#[derive(Debug)]
pub struct PrintResponse {
    pub handle: RequestHandle,
    pub outcome: PrintOutcome,
}
impl Event for PrintResponse {}

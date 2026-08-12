use app::Event;

pub type RequestHandle = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum InputCaptureCapability {
    Keyboard = 1,
    Pointer = 2,
    Touchscreen = 4,
}

impl InputCaptureCapability {
    pub const ALL: u32 = (InputCaptureCapability::Keyboard as u32)
        | (InputCaptureCapability::Pointer as u32)
        | (InputCaptureCapability::Touchscreen as u32);
}

#[derive(Debug, Clone)]
pub enum RequestKind {
    Start,
}

#[derive(Debug)]
pub enum InputCaptureRequest {
    Start {
        handle: RequestHandle,
        session_handle: String,
        app_id: String,
        capabilities: u32,
    },
    Close {
        handle: RequestHandle,
    },
}
impl Event for InputCaptureRequest {}

#[derive(Debug, Clone)]
pub struct InputCaptureResponse {
    pub handle: RequestHandle,
    pub outcome: InputCaptureOutcome,
}
impl Event for InputCaptureResponse {}

#[derive(Debug, Clone)]
pub enum InputCaptureOutcome {
    Granted { capabilities: u32 },
    Denied,
}

#[derive(Debug, Clone)]
pub struct InputCaptureSession {
    pub capabilities: u32,
    pub app_id: String,
    pub closed: bool,
}

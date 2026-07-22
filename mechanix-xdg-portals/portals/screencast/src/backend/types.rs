use app::Event;
use zbus::zvariant::{DeserializeDict, OwnedValue, SerializeDict, Type};

pub type RequestHandle = String;
pub type RestoreData = (String, u32, OwnedValue);

// --- ScreenCast Enums & Bitmask Types ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SourceType {
    Monitor = 1,
    Window = 2,
    Virtual = 4,
}

impl SourceType {
    pub const ALL: u32 = (SourceType::Monitor as u32)
        | (SourceType::Window as u32)
        | (SourceType::Virtual as u32);

    pub fn bits(self) -> u32 {
        self as u32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum CursorMode {
    Hidden = 1,
    Embedded = 2,
    Metadata = 4,
}

impl CursorMode {
    pub const ALL: u32 = (CursorMode::Hidden as u32)
        | (CursorMode::Embedded as u32)
        | (CursorMode::Metadata as u32);

    pub fn bits(self) -> u32 {
        self as u32
    }

    pub fn from_bits(bits: u32) -> Option<Self> {
        match bits {
            1 => Some(Self::Hidden),
            2 => Some(Self::Embedded),
            4 => Some(Self::Metadata),
            _ => None,
        }
    }
}

#[derive(DeserializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct CreateSessionOptions {
    pub session_handle_token: Option<String>,
}

#[derive(SerializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct CreateSessionResults {}

#[derive(DeserializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct SelectSourcesOptions {
    pub types: Option<u32>,
    pub multiple: Option<bool>,
    pub cursor_mode: Option<u32>,
    pub restore_data: Option<RestoreData>,
    pub persist_mode: Option<u32>,
}

#[derive(SerializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct SelectSourcesResults {
    pub restore_data: Option<RestoreData>,
}

#[derive(DeserializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct StartOptions {
    pub handle_token: Option<String>,
}

#[derive(SerializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct StreamOptions {
    pub size: Option<(i32, i32)>,
    pub position: Option<(i32, i32)>,
    pub source_type: Option<u32>,
    pub mapping_id: Option<String>,
    pub pipewire_serial: Option<u64>,
    pub restore_data: Option<RestoreData>,
}

#[derive(SerializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct StartResults {
    pub streams: Option<Vec<(u32, StreamOptions)>>,
    pub persist_mode: Option<u32>,
    pub restore_data: Option<RestoreData>,
}

#[derive(DeserializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct OpenPipeWireRemoteOptions {}

#[derive(Debug, Clone)]
pub enum RequestKind {
    Start,
}

#[derive(Debug)]
pub enum ScreenCastRequest {
    Start {
        handle: RequestHandle,
        session_handle: String,
        app_id: String,
        types: u32,
        cursor_mode: Option<CursorMode>,
        multiple: bool,
    },
    Close {
        handle: RequestHandle,
    },
}
impl Event for ScreenCastRequest {}

#[derive(Debug)]
pub struct ScreenCastResponse {
    pub handle: RequestHandle,
    pub outcome: ScreenCastOutcome,
}
impl Event for ScreenCastResponse {}

#[derive(Debug, Clone)]
pub enum ScreenCastOutcome {
    Granted { selected_type: u32 },
    Denied,
}

#[derive(Debug, Clone)]
pub struct PersistedCaptureSources;

#[derive(Debug, Clone)]
pub struct ScreencastSession {
    pub cursor_mode: Option<CursorMode>,
    pub multiple: bool,
    pub source_types: u32,
    pub persisted_capture_sources: Option<PersistedCaptureSources>,
    pub closed: bool,
}

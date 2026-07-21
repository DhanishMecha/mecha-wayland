use app::Event;
use zbus::zvariant::{ DeserializeDict, SerializeDict, Type };

pub type RequestHandle = String;
pub type FilterRule = (u32, String);
pub type FileFilter = (String, Vec<FilterRule>);

/// Options supported by the OpenFile method.
#[derive(DeserializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct OpenFileOptions {
    pub accept_label: Option<String>,
    pub modal: Option<bool>,
    pub multiple: Option<bool>,
    pub directory: Option<bool>,
    pub filters: Option<Vec<FileFilter>>,
    pub current_filter: Option<FileFilter>,
    pub current_folder: Option<Vec<u8>>,
}

/// Options supported by the SaveFile method.
#[derive(DeserializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct SaveFileOptions {
    pub accept_label: Option<String>,
    pub modal: Option<bool>,
    pub filters: Option<Vec<FileFilter>>,
    pub current_filter: Option<FileFilter>,
    pub current_name: Option<String>,
    pub current_folder: Option<Vec<u8>>,
    pub current_file: Option<Vec<u8>>,
}

/// Options supported by the SaveFiles method.
#[derive(DeserializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct SaveFilesOptions {
    pub accept_label: Option<String>,
    pub modal: Option<bool>,
    pub current_folder: Option<Vec<u8>>,
    pub files: Option<Vec<Vec<u8>>>,
}

/// Results returned by the FileChooser methods.
#[derive(SerializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct FileChooserResults {
    pub uris: Option<Vec<String>>,
}

/// Emitted by the D-Bus backend; consumed by the FileChooser UI.
#[derive(Debug)]
pub enum FileChooserRequest {
    OpenFile {
        handle: RequestHandle,
        title: String,
        options: OpenFileOptions,
    },
    SaveFile {
        handle: RequestHandle,
        title: String,
        options: SaveFileOptions,
    },
    SaveFiles {
        handle: RequestHandle,
        title: String,
        options: SaveFilesOptions,
    },
    Close {
        handle: RequestHandle,
    },
}
impl Event for FileChooserRequest {}

/// Emitted by the FileChooser UI when the user is done; consumed by the backend to reply.
#[derive(Debug)]
pub struct FileChooserResponse {
    pub handle: RequestHandle,
    pub outcome: FileChooserOutcome,
}
impl Event for FileChooserResponse {}

#[derive(Debug, Clone)]
pub enum FileChooserOutcome {
    Selected(Vec<String>), // file:// URIs
    Cancelled,
}

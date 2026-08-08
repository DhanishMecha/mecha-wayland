use app::Event;
use zbus::zvariant::{DeserializeDict, Type};

pub type RequestHandle = String;

#[derive(DeserializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "kebab-case")]
pub struct WallpaperOptions {
    pub show_preview: Option<bool>,
    pub set_on: Option<String>,
}

#[derive(Debug)]
pub enum WallpaperRequest {
    SetWallpaper {
        handle: RequestHandle,
        app_id: String,
        uri: String,
        show_preview: bool,
        set_on: String,
    },
    Close {
        handle: RequestHandle,
    },
}
impl Event for WallpaperRequest {}

#[derive(Debug)]
pub struct WallpaperResponse {
    pub handle: RequestHandle,
    pub outcome: WallpaperOutcome,
}
impl Event for WallpaperResponse {}

#[derive(Debug, Clone)]
pub enum WallpaperOutcome {
    Granted,
    Denied,
}

use app::Event;
use zbus::zvariant::OwnedValue;

pub type RequestHandle = String;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum DynamicLauncherOutcome {
    Granted,
    Denied,
}

#[derive(Debug, Clone)]
pub struct DynamicLauncherRequestInfo {
    pub handle: RequestHandle,
    pub app_id: String,
    pub name: String,
    pub icon_v: OwnedValue,
    pub launcher_type: u32,
    pub target: Option<String>,
}

#[derive(Debug, Clone)]
pub enum DynamicLauncherRequest {
    PrepareInstall(DynamicLauncherRequestInfo),
    Close { handle: RequestHandle },
}
impl Event for DynamicLauncherRequest {}

#[derive(Debug, Clone)]
pub struct DynamicLauncherResponse {
    pub handle: RequestHandle,
    pub outcome: DynamicLauncherOutcome,
}
impl Event for DynamicLauncherResponse {}

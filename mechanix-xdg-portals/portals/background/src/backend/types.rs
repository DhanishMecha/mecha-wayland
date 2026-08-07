use serde::{Deserialize, Serialize};
use app::Event;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BackgroundOutcome {
    Forbid,             // 0
    Allow,              // 1
    AllowThisInstance,  // 2
}

impl BackgroundOutcome {
    pub fn to_u32(&self) -> u32 {
        match self {
            Self::Forbid => 0,
            Self::Allow => 1,
            Self::AllowThisInstance => 2,
        }
    }
}

pub type RequestHandle = String;

#[derive(Clone, Debug)]
pub enum BackgroundRequest {
    NotifyBackground {
        handle: RequestHandle,
        app_id: String,
        name: String,
    },
    Close {
        handle: RequestHandle,
    },
}
impl Event for BackgroundRequest {}

#[derive(Clone, Debug)]
pub struct BackgroundResponse {
    pub handle: RequestHandle,
    pub outcome: BackgroundOutcome,
}
impl Event for BackgroundResponse {}


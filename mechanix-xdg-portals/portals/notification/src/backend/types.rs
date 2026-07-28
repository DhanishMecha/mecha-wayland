use std::collections::HashMap;
use zbus::zvariant::OwnedValue;
use app::Event;

#[derive(Debug, Clone)]
pub struct NotificationButton {
    pub label: String,
    pub action: String,
    pub target: Option<OwnedValue>,
}

#[derive(Debug, Clone)]
pub struct NotificationInfo {
    pub title: String,
    pub body: String,
    pub icon: Option<String>,
    pub icon_value: Option<OwnedValue>, // Raw D-Bus variant for complex icons
    pub priority: String, // "low", "normal", "high", "urgent"
    pub default_action: Option<String>,
    pub default_action_target: Option<OwnedValue>,
    pub buttons: Vec<NotificationButton>,
    pub markup_body: Option<String>,
    pub sound: Option<OwnedValue>,
    pub display_hints: Vec<String>,
    pub category: Option<String>,
}

impl NotificationInfo {
    pub fn from_map(map: HashMap<String, OwnedValue>) -> Self {
        super::helpers::parse_notification(map)
    }
}

#[derive(Debug, Clone)]
pub enum NotificationRequest {
    Add {
        app_id: String,
        id: String,
        info: NotificationInfo,
    },
    Remove {
        app_id: String,
        id: String,
    },
}
impl Event for NotificationRequest {}

#[derive(Debug, Clone)]
pub struct NotificationResponse {
    pub app_id: String,
    pub id: String,
    pub action: String,
    pub parameter: Vec<zbus::zvariant::OwnedValue>,
}
impl Event for NotificationResponse {}

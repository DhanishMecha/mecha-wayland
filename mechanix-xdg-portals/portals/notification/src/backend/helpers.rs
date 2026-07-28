use std::collections::HashMap;
use zbus::zvariant::{OwnedValue, Value};
use super::types::{NotificationButton, NotificationInfo};

/// Extracts and converts a string value from a D-Bus map, removing it from the map.
pub fn remove_string(map: &mut HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    map.remove(key).and_then(|v| String::try_from(v).ok())
}

/// Parses a raw D-Bus dictionary representing notification data into a `NotificationInfo` struct.
pub fn parse_notification(mut map: HashMap<String, OwnedValue>) -> NotificationInfo {
    let title = remove_string(&mut map, "title").unwrap_or_default();
    let body = remove_string(&mut map, "body").unwrap_or_default();

    let icon_value = map.remove("icon");
    // TODO: icon render is not supported
    let icon = icon_value.as_ref().and_then(|v| match &**v {
        Value::Str(s) => Some(s.to_string()),
        _ => None,
    });

    let priority = remove_string(&mut map, "priority").unwrap_or_else(|| "normal".to_string());
    let default_action = remove_string(&mut map, "default-action");
    let default_action_target = map.remove("default-action-target");
    let markup_body = remove_string(&mut map, "markup-body");
    let sound = map.remove("sound");

    let mut display_hints = Vec::new();
    if let Some(hint_val) = map.remove("display-hint") {
        if let Ok(hints) = Vec::<String>::try_from(hint_val) {
            display_hints = hints;
        }
    }

    let category = remove_string(&mut map, "category");

    let mut buttons = Vec::new();
    if let Some(buttons_val) = map.remove("buttons") {
        if let Ok(arr) = Vec::<HashMap<String, OwnedValue>>::try_from(buttons_val) {
            for mut button_map in arr {
                let label = remove_string(&mut button_map, "label").unwrap_or_default();
                let action = remove_string(&mut button_map, "action").unwrap_or_default();
                let target = button_map.remove("target");

                if !label.is_empty() && !action.is_empty() {
                    buttons.push(NotificationButton {
                        label,
                        action,
                        target,
                    });
                }
            }
        }
    }

    NotificationInfo {
        title,
        body,
        icon,
        icon_value,
        priority,
        default_action,
        default_action_target,
        buttons,
        markup_body,
        sound,
        display_hints,
        category,
    }
}

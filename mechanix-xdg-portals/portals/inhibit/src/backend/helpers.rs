use std::collections::HashMap;
use zbus::zvariant::OwnedValue;

use super::types::InhibitOptions;

/// Parse the options vardict from the Inhibit D-Bus call.
pub fn parse_inhibit_options(options: &HashMap<String, OwnedValue>) -> InhibitOptions {
    let reason = options.get("reason").and_then(|v| {
        if let Ok(s) = String::try_from(v.clone()) {
            Some(s)
        } else {
            None
        }
    });

    InhibitOptions { reason }
}

/// Human-readable description of the inhibit flags bitmask.
pub fn describe_flags(flags: u32) -> String {
    let mut parts = Vec::new();
    if flags & 1 != 0 {
        parts.push("Logout");
    }
    if flags & 2 != 0 {
        parts.push("UserSwitch");
    }
    if flags & 4 != 0 {
        parts.push("Suspend");
    }
    if flags & 8 != 0 {
        parts.push("Idle");
    }
    if parts.is_empty() {
        "None".to_string()
    } else {
        parts.join("|")
    }
}

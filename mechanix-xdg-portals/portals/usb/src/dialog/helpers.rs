use std::collections::HashMap;
use zbus::zvariant::{OwnedValue, Value};

use crate::backend::types::UsbDeviceUiInfo;

fn get_string(v: &OwnedValue) -> Option<String> {
    if let Ok(s) = String::try_from(v.clone()) {
        return Some(s);
    }
    let inner: Value<'_> = Value::from(v.clone());
    if let Value::Value(boxed) = inner {
        String::try_from(*boxed).ok()
    } else {
        None
    }
}
fn get_bool(v: &OwnedValue) -> Option<bool> {
    if let Ok(b) = bool::try_from(v.clone()) {
        return Some(b);
    }
    let inner: Value<'_> = Value::from(v.clone());
    if let Value::Value(boxed) = inner {
        bool::try_from(*boxed).ok()
    } else {
        None
    }
}

/// Look up a string in a `HashMap<String, OwnedValue>` by key.
fn get_string_prop(map: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    map.get(key).and_then(get_string)
}
fn unwrap_dict(v: &OwnedValue) -> HashMap<String, OwnedValue> {
    if let Ok(map) = HashMap::<String, OwnedValue>::try_from(v.clone()) {
        return map;
    }
    let inner: Value<'_> = Value::from(v.clone());
    if let Value::Value(boxed) = inner {
        HashMap::<String, OwnedValue>::try_from(*boxed).unwrap_or_default()
    } else {
        HashMap::default()
    }
}

/// Arguments:
/// - `dev_id`      — the device identifier string
/// - `info_dict`   — `a{sv}` dict with `"device-file"` and `"properties"` keys
/// - `access_opts` — `a{sv}` dict with per-device access options (e.g. `"writable"`)
pub fn parse_device(
    dev_id: &str,
    info_dict: &HashMap<String, OwnedValue>,
    access_opts: &HashMap<String, OwnedValue>,
) -> UsbDeviceUiInfo {
    // ---- Resolve display name from vendor / model properties ----
    let properties = info_dict
        .get("properties")
        .map(unwrap_dict)
        .unwrap_or_default();

    let vendor = get_string_prop(&properties, "ID_VENDOR_FROM_DATABASE")
        .or_else(|| get_string_prop(&properties, "ID_VENDOR"));
    let model = get_string_prop(&properties, "ID_MODEL_FROM_DATABASE")
        .or_else(|| get_string_prop(&properties, "ID_MODEL"));

    let name = match (vendor, model) {
        (Some(v), Some(m)) => format!("{v} {m}"),
        (None, Some(m)) => m,
        (Some(v), None) => format!("{v} Device"),
        (None, None) => format!("USB Device {dev_id}"),
    };

    // ---- Resolve device file path ----
    let dev_file = info_dict
        .get("device-file")
        .and_then(get_string)
        .unwrap_or_else(|| "Unknown path".to_string());

    // ---- Resolve access mode ----
    let is_writable = access_opts
        .get("writable")
        .and_then(get_bool)
        .unwrap_or(false);

    let access_str = if is_writable {
        "Read/Write"
    } else {
        "Read-only"
    };
    let details = format!("{dev_file} ({access_str})");

    UsbDeviceUiInfo {
        device_id: dev_id.to_string(),
        name,
        details,
        selected: true,
        access_options: access_opts.clone(),
    }
}

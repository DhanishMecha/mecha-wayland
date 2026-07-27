use std::collections::HashMap;
use zbus::zvariant::{OwnedValue, StructureBuilder, Value};
use super::types::{
    SettingsMap, APPEARANCE_NS,
    KEY_COLOR_SCHEME, KEY_CONTRAST, KEY_REDUCED_MOTION, KEY_ACCENT_COLOR,
    ColorScheme, Contrast, ReducedMotion, AccentColor
};

pub const DEFAULT_SETTINGS_TOML: &str = r#"[org.freedesktop.appearance]
color-scheme = 0
contrast = 0
accent-color = [0.38, 0.49, 0.85]
reduced-motion = 0
"#;

// Checks if a namespace matches a filter (with support for wildcards).
pub fn namespace_matches(filter: &str, namespace: &str) -> bool {
    if filter.is_empty() {
        return true;
    }
    if let Some(prefix) = filter.strip_suffix('*') {
        namespace.starts_with(prefix)
    } else {
        namespace == filter
    }
}

// Looks up a setting value by namespace and key.
pub fn lookup<'a>(store: &'a SettingsMap, namespace: &str, key: &str) -> Option<&'a OwnedValue> {
    store.get(namespace)?.get(key)
}

// Filters the settings map based on a list of namespace patterns.
pub fn filter_namespaces(
    store: &SettingsMap,
    filters: &[String]
) -> HashMap<String, HashMap<String, OwnedValue>> {
    let all = filters.is_empty() || filters.iter().any(|f| f.is_empty());
    if all {
        return store
            .iter()
            .map(|(ns, kv)| (ns.clone(), kv.clone()))
            .collect();
    }

    store
        .iter()
        .filter(|(ns, _)| filters.iter().any(|f| namespace_matches(f, ns)))
        .map(|(ns, kv)| (ns.clone(), kv.clone()))
        .collect()
}

// Converts a TOML value to a DBus-compatible OwnedValue.
pub fn toml_val_to_owned_value(val: &toml::Value) -> Option<OwnedValue> {
    match val {
        toml::Value::Boolean(b) => {
            OwnedValue::try_from(Value::Bool(*b)).ok()
        }
        toml::Value::Integer(i) => {
            OwnedValue::try_from(Value::I64(*i)).ok()
        }
        toml::Value::Float(f) => {
            OwnedValue::try_from(Value::F64(*f)).ok()
        }
        toml::Value::String(s) => {
            OwnedValue::try_from(Value::Str(s.as_str().into())).ok()
        }
        toml::Value::Array(arr) => {
            let mut values = Vec::new();
            for item in arr {
                if let Some(ov) = toml_val_to_owned_value(item) {
                    values.push(Value::from(ov));
                }
            }
            if !values.is_empty() {
                let mut builder = StructureBuilder::new();
                for v in &values {
                    builder = builder.add_field(v.clone());
                }
                if let Ok(s) = builder.build() {
                    return OwnedValue::try_from(Value::Structure(s)).ok();
                }
            }
            None
        }
        _ => None,
    }
}

// Recursively flattens a nested TOML table structure into the flat SettingsMap.
fn flatten_toml_table(
    table: &toml::map::Map<String, toml::Value>,
    current_prefix: &str,
    store: &mut SettingsMap,
) {
    let mut has_values = false;
    for (_, val) in table {
        if !val.is_table() {
            has_values = true;
            break;
        }
    }
    
    if has_values && !current_prefix.is_empty() {
        let store_ns = store.entry(current_prefix.to_string()).or_default();
        for (key, val) in table {
            if !val.is_table() {
                if current_prefix == APPEARANCE_NS {
                    match key.as_str() {
                        KEY_COLOR_SCHEME => {
                            if let Some(i) = val.as_integer() {
                                store_ns.insert(key.clone(), ColorScheme::from_u32(i as u32).into());
                            }
                        }
                        KEY_CONTRAST => {
                            if let Some(i) = val.as_integer() {
                                store_ns.insert(key.clone(), Contrast::from_u32(i as u32).into());
                            }
                        }
                        KEY_REDUCED_MOTION => {
                            if let Some(i) = val.as_integer() {
                                store_ns.insert(key.clone(), ReducedMotion::from_u32(i as u32).into());
                            }
                        }
                        KEY_ACCENT_COLOR => {
                            if let Some(arr) = val.as_array() {
                                if arr.len() == 3 {
                                    let r = arr[0].as_float().or_else(|| arr[0].as_integer().map(|i| i as f64)).unwrap_or(0.0);
                                    let g = arr[1].as_float().or_else(|| arr[1].as_integer().map(|i| i as f64)).unwrap_or(0.0);
                                    let b = arr[2].as_float().or_else(|| arr[2].as_integer().map(|i| i as f64)).unwrap_or(0.0);
                                    store_ns.insert(key.clone(), AccentColor(r, g, b).into());
                                }
                            }
                        }
                        _ => {
                            if let Some(v) = toml_val_to_owned_value(val) {
                                store_ns.insert(key.clone(), v);
                            }
                        }
                    }
                } else {
                    if let Some(v) = toml_val_to_owned_value(val) {
                        store_ns.insert(key.clone(), v);
                    }
                }
            }
        }
    }
    
    for (key, val) in table {
        if let Some(sub_table) = val.as_table() {
            let next_prefix = if current_prefix.is_empty() {
                key.clone()
            } else {
                format!("{}.{}", current_prefix, key)
            };
            flatten_toml_table(sub_table, &next_prefix, store);
        }
    }
}

// Loads and parses the settings TOML file into a SettingsMap.
pub fn load_from_file(path: &std::path::Path) -> Result<SettingsMap, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read settings file: {e}"))?;
    
    let toml: toml::Value = content.parse()
        .map_err(|e| format!("Failed to parse settings TOML: {e}"))?;
        
    let table = toml.as_table()
        .ok_or_else(|| "Settings TOML root must be a table".to_string())?;
        
    let mut store = default_settings();
    
    flatten_toml_table(table, "", &mut store);
    
    Ok(store)
}

// --- Default store -----------------------------------------------------------

pub fn default_settings() -> SettingsMap {
    let mut store: SettingsMap = HashMap::new();

    let mut appearance: HashMap<String, OwnedValue> = HashMap::new();
    appearance.insert(KEY_COLOR_SCHEME.to_string(), ColorScheme::NoPreference.into());
    appearance.insert(KEY_CONTRAST.to_string(), Contrast::NoPreference.into());
    appearance.insert(KEY_ACCENT_COLOR.to_string(), AccentColor(0.38, 0.49, 0.85).into());
    appearance.insert(KEY_REDUCED_MOTION.to_string(), ReducedMotion::NoPreference.into());

    store.insert(APPEARANCE_NS.to_string(), appearance);
    store
}


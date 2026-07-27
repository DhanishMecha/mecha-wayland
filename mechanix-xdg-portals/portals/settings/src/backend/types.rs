use std::collections::HashMap;

use zbus::zvariant::{OwnedValue, StructureBuilder, Value};

pub type SettingsMap = HashMap<String, HashMap<String, OwnedValue>>;

// --- Namespace & key constants -----------------------------------------------

pub const APPEARANCE_NS: &str = "org.freedesktop.appearance";
pub const KEY_COLOR_SCHEME: &str = "color-scheme";
pub const KEY_CONTRAST: &str = "contrast";
pub const KEY_ACCENT_COLOR: &str = "accent-color";
pub const KEY_REDUCED_MOTION: &str = "reduced-motion";

// --- Typed setting values -----------------------------------------------------

/// `org.freedesktop.appearance` → `color-scheme`
/// 0 = no-preference, 1 = prefer-dark, 2 = prefer-light
#[repr(u32)]
pub enum ColorScheme {
    NoPreference = 0,
    PreferDark   = 1,
    PreferLight  = 2,
}

impl ColorScheme {
    pub fn from_u32(v: u32) -> Self {
        match v {
            1 => ColorScheme::PreferDark,
            2 => ColorScheme::PreferLight,
            _ => ColorScheme::NoPreference,
        }
    }
}

impl From<ColorScheme> for OwnedValue {
    fn from(v: ColorScheme) -> Self {
        OwnedValue::try_from(Value::U32(v as u32)).unwrap()
    }
}

/// `org.freedesktop.appearance` → `contrast`
/// 0 = no-preference, 1 = high-contrast
#[repr(u32)]
pub enum Contrast {
    NoPreference = 0,
    High         = 1,
}

impl Contrast {
    pub fn from_u32(v: u32) -> Self {
        match v {
            1 => Contrast::High,
            _ => Contrast::NoPreference,
        }
    }
}

impl From<Contrast> for OwnedValue {
    fn from(v: Contrast) -> Self {
        OwnedValue::try_from(Value::U32(v as u32)).unwrap()
    }
}

/// `org.freedesktop.appearance` → `reduced-motion`
/// 0 = no-preference, 1 = reduced-motion
#[repr(u32)]
pub enum ReducedMotion {
    NoPreference  = 0,
    ReducedMotion = 1,
}

impl ReducedMotion {
    pub fn from_u32(v: u32) -> Self {
        match v {
            1 => ReducedMotion::ReducedMotion,
            _ => ReducedMotion::NoPreference,
        }
    }
}

impl From<ReducedMotion> for OwnedValue {
    fn from(v: ReducedMotion) -> Self {
        OwnedValue::try_from(Value::U32(v as u32)).unwrap()
    }
}

/// `org.freedesktop.appearance` → `accent-color`
/// Normalized (r, g, b) components in the range 0.0–1.0.
pub struct AccentColor(pub f64, pub f64, pub f64);

impl From<AccentColor> for OwnedValue {
    fn from(v: AccentColor) -> Self {
        let s = StructureBuilder::new()
            .add_field(Value::F64(v.0))
            .add_field(Value::F64(v.1))
            .add_field(Value::F64(v.2))
            .build()
            .expect("AccentColor structure build failed");
        OwnedValue::try_from(Value::Structure(s)).unwrap()
    }
}

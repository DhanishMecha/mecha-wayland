//! The three interfaces the generator leaves out: the display, the registry
//! and the callback. Their wire format is fixed and their requests are the
//! connection's own.

pub mod client;

use crate::Interface;

// ── Read helpers shared with the generated parse functions ────────────────────

pub(crate) fn read_u32(data: &[u8], offset: &mut usize) -> Option<u32> {
    let bytes = data.get(*offset..*offset + 4)?;
    *offset += 4;
    Some(u32::from_ne_bytes(bytes.try_into().unwrap()))
}

pub(crate) fn read_string(data: &[u8], offset: &mut usize) -> Option<String> {
    let len = read_u32(data, offset)? as usize;
    let padded = (len + 3) & !3;
    let raw = data.get(*offset..*offset + padded)?;
    *offset += padded;
    let s = std::str::from_utf8(raw.get(..len.saturating_sub(1))?).ok()?;
    Some(s.to_owned())
}

#[derive(Debug)]
pub struct WlDisplay;
impl Interface for WlDisplay {
    const NAME: &'static str = "wl_display";
    const VERSION: u32 = 1;
}

#[derive(Debug)]
pub struct WlCallback;
impl Interface for WlCallback {
    const NAME: &'static str = "wl_callback";
    const VERSION: u32 = 1;
}

#[derive(Debug)]
pub struct WlRegistry;
impl Interface for WlRegistry {
    const NAME: &'static str = "wl_registry";
    const VERSION: u32 = 1;
}

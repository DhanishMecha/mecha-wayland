use app::Event;
use std::collections::HashMap;
use zbus::zvariant::{DeserializeDict, OwnedValue, SerializeDict, Type, Value};

pub type RequestHandle = String;
#[derive(DeserializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct UsbAcquireDevicesOptions {}

#[derive(SerializeDict, Type, Debug, Default)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct UsbResultsDict {
    pub devices: Option<Vec<(String, HashMap<String, Value<'static>>)>>,
}

// UI info — one row per device shown in the dialog

#[derive(Debug, Clone)]
pub struct UsbDeviceUiInfo {
    pub device_id: String,
    pub name: String,
    pub details: String,
    pub selected: bool,
    pub access_options: HashMap<String, OwnedValue>,
}

#[derive(Debug, Clone)]
pub struct UsbRequestInfo {
    pub handle: RequestHandle,
    pub app_id: String,
    /// Each device is `(device_id, device_info_dict, access_options_dict)`.
    pub devices: Vec<(
        String,
        HashMap<String, OwnedValue>,
        HashMap<String, OwnedValue>,
    )>,
}

#[derive(Debug, Clone)]
pub enum UsbRequest {
    AcquireDevices(UsbRequestInfo),
    Close { handle: RequestHandle },
}
impl Event for UsbRequest {}

#[derive(Debug, Clone)]
pub enum UsbOutcome {
    Granted(Vec<(String, HashMap<String, OwnedValue>)>),
    Denied,
}

#[derive(Debug, Clone)]
pub struct UsbResponse {
    pub handle: RequestHandle,
    pub outcome: UsbOutcome,
}
impl Event for UsbResponse {}

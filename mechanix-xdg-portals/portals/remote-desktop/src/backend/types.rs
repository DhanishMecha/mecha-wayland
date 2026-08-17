use app::Event;
use zbus::zvariant::{DeserializeDict, OwnedValue, SerializeDict, Type};

pub type RequestHandle = String;
pub type RestoreData = (String, u32, OwnedValue);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum DeviceType {
    Keyboard = 1,
    Pointer = 2,
    Touchscreen = 4,
}

impl DeviceType {
    pub const ALL: u32 = (DeviceType::Keyboard as u32)
        | (DeviceType::Pointer as u32)
        | (DeviceType::Touchscreen as u32);

    pub fn bits(self) -> u32 {
        self as u32
    }
}

/// How a remote desktop session should persist across application runs.
///
/// - `NoPersist`  (0): Session does not persist (default).
/// - `WhileRunning` (1): Permissions persist as long as the app is running.
/// - `UntilRevoked` (2): Permissions persist until explicitly revoked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum PersistMode {
    #[default]
    NoPersist = 0,
    WhileRunning = 1,
    UntilRevoked = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum KeyState {
    Released = 0,
    Pressed = 1,
}

/// Discrete scroll axis for `NotifyPointerAxisDiscrete`.
///
/// - `Vertical`   (0)
/// - `Horizontal` (1)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ScrollAxis {
    Vertical = 0,
    Horizontal = 1,
}

/// Options for `CreateSession`.
///
/// Supported keys:
///   - `session_handle_token` (s): Token for the session object path.
#[derive(DeserializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct CreateSessionOptions {
    pub session_handle_token: Option<String>,
}

/// Results returned by `CreateSession`.
///
/// Currently no keys are defined by the spec.
#[derive(SerializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct CreateSessionResults {}

/// Options for `SelectDevices`.
///
/// Supported keys:
///   - `types` (u): Bitmask of `DeviceType` values. Defaults to all.
///   - `restore_data` ((suv)): Data from a previous session to restore (v2).
///   - `persist_mode` (u): See `PersistMode` (v2).
#[derive(DeserializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct SelectDevicesOptions {
    pub types: Option<u32>,
    pub restore_data: Option<RestoreData>,
    pub persist_mode: Option<u32>,
}

/// Results returned by `SelectDevices`.
///
/// Currently no keys are defined by the spec.
#[derive(SerializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct SelectDevicesResults {}

/// Options for `Start`.
///
/// Supported keys:
///   - `handle_token` (s): Token for the request object path.
///   - `persist_mode` (u): See `PersistMode` (v2).
#[derive(DeserializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct StartOptions {
    pub handle_token: Option<String>,
    pub persist_mode: Option<u32>,
}

/// Stream properties returned within `StartResults`.
///
/// Equivalent to `StreamOptions` in the ScreenCast portal.
#[derive(SerializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct StreamOptions {
    pub size: Option<(i32, i32)>,
    pub position: Option<(i32, i32)>,
    pub source_type: Option<u32>,
    pub mapping_id: Option<String>,
    pub pipewire_serial: Option<u64>,
    pub restore_data: Option<RestoreData>,
}

/// Results returned by `Start`.
///
/// Keys:
///   - `devices` (u): The device types that were granted.
///   - `clipboard_enabled` (b): Whether clipboard sharing is active (v2).
///   - `streams` (a(ua{sv})): PipeWire streams for associated screen cast.
///   - `restore_data` ((suv)): Data for future session restore (v2).
#[derive(SerializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct StartResults {
    pub devices: Option<u32>,
    pub clipboard_enabled: Option<bool>,
    pub streams: Option<Vec<(u32, StreamOptions)>>,
    pub restore_data: Option<RestoreData>,
}

/// Options for `NotifyPointerMotion`. Currently no supported keys.
#[derive(DeserializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct NotifyPointerMotionOptions {}

/// Options for `NotifyPointerMotionAbsolute`. Currently no supported keys.
#[derive(DeserializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct NotifyPointerMotionAbsoluteOptions {}

/// Options for `NotifyPointerButton`. Currently no supported keys.
#[derive(DeserializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct NotifyPointerButtonOptions {}

/// Options for `NotifyPointerAxis`.
///
/// Supported keys:
///   - `finish` (b): True if this is the last event in an axis series (e.g.
///     fingers lifted from touchpad). Defaults to false.
#[derive(DeserializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct NotifyPointerAxisOptions {
    pub finish: Option<bool>,
}

/// Options for `NotifyPointerAxisDiscrete`. Currently no supported keys.
#[derive(DeserializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct NotifyPointerAxisDiscreteOptions {}

/// Options for `NotifyKeyboardKeycode`. Currently no supported keys.
#[derive(DeserializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct NotifyKeyboardKeycodeOptions {}

/// Options for `NotifyKeyboardKeysym`. Currently no supported keys.
#[derive(DeserializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct NotifyKeyboardKeysymOptions {}

/// Options for `NotifyTouchDown`. Currently no supported keys.
#[derive(DeserializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct NotifyTouchDownOptions {}

/// Options for `NotifyTouchMotion`. Currently no supported keys.
#[derive(DeserializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct NotifyTouchMotionOptions {}

/// Options for `NotifyTouchUp`. Currently no supported keys.
#[derive(DeserializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct NotifyTouchUpOptions {}

/// Options for `ConnectToEIS`. Currently no supported keys.
#[derive(DeserializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct ConnectToEISOptions {}

/// The kind of pending request stored while waiting for user interaction.
#[derive(Debug, Clone)]
pub enum RequestKind {
    Start,
}

/// Tracks configured state of an active remote desktop session.
#[derive(Debug, Clone)]
pub struct RemoteDesktopSession {
    /// Bitmask of requested device types (see `DeviceType`).
    /// Starts as `DeviceType::ALL` until `SelectDevices` narrows it down.
    pub device_types: u32,
    /// Persist mode chosen during `SelectDevices`.
    pub persist_mode: PersistMode,
    /// `true` once `SelectDevices` has successfully been called for this session.
    /// Calls to `Start` must be rejected if this is still `false`.
    pub devices_selected: bool,
    /// Whether the session has been closed / invalidated.
    pub closed: bool,
}

/// Event sent from the UI layer back to the backend to signal user decision.
#[derive(Debug)]
pub struct RemoteDesktopRequest {
    pub handle: RequestHandle,
    pub session_handle: String,
    pub app_id: String,
    pub device_types: u32,
}
impl Event for RemoteDesktopRequest {}

/// Response from the UI layer once the user has accepted or denied access.
#[derive(Debug)]
pub struct RemoteDesktopResponse {
    pub handle: RequestHandle,
    pub outcome: RemoteDesktopOutcome,
}
impl Event for RemoteDesktopResponse {}

/// Outcome of a user-interaction dialog for a remote desktop start request.
#[derive(Debug, Clone)]
pub enum RemoteDesktopOutcome {
    /// User granted remote desktop access; `granted_devices` is the bitmask.
    Granted { granted_devices: u32 },
    /// User denied the request.
    Denied,
}

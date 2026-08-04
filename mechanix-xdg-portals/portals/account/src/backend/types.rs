use app::Event;
use zbus::zvariant::{DeserializeDict, SerializeDict, Type};

pub type RequestHandle = String;

#[derive(DeserializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}", rename_all = "snake_case")]
pub struct AccountOptions {
    pub reason: Option<String>,
}

/// Results returned to the caller on success.
#[derive(SerializeDict, Type, Debug, Default, Clone)]
#[zvariant(signature = "a{sv}")]
pub struct AccountResults {
    pub id: Option<String>,
    pub name: Option<String>,
    pub image: Option<String>,
}

/// Resolved user information read from the system.
#[derive(Debug, Clone)]
pub struct UserInfo {
    pub id: String,
    pub name: String,
    pub image: String,
}

/// Event dispatched from the backend to the UI layer to open the consent dialog.
#[derive(Debug)]
pub enum AccountRequest {
    GetUserInformation {
        handle: RequestHandle,
        app_id: String,
        reason: Option<String>,
        user_info: UserInfo,
    },
    Close {
        handle: RequestHandle,
    },
}
impl Event for AccountRequest {}

/// Event dispatched from the UI layer back to the backend with the user's decision.
#[derive(Debug)]
pub struct AccountResponse {
    pub handle: RequestHandle,
    pub outcome: AccountOutcome,
}
impl Event for AccountResponse {}

/// The user's choice in the consent dialog.
#[derive(Debug, Clone)]
pub enum AccountOutcome {
    Granted,
    Denied,
}

use dbus::dbus_interface;
use zbus::zvariant::{OwnedFd, OwnedObjectPath};

use super::types::{
    CreateSessionOptions, CreateSessionResults, OpenPipeWireRemoteOptions, SelectSourcesOptions,
    SelectSourcesResults, StartOptions, StartResults,
};

pub const SCREENCAST_IFACE: &str = "org.freedesktop.impl.portal.ScreenCast";
pub const SCREENCAST_VERSION: u32 = 6;

dbus_interface!(pub ScreenCastIface = SCREENCAST_IFACE;
    method CreateSession(
        handle: OwnedObjectPath,
        session_handle: OwnedObjectPath,
        app_id: String,
        options: CreateSessionOptions,
    ) -> (response: u32, results: CreateSessionResults);
    method SelectSources(
        handle: OwnedObjectPath,
        session_handle: OwnedObjectPath,
        app_id: String,
        options: SelectSourcesOptions,
    ) -> (response: u32, results: SelectSourcesResults);
    method Start(
        handle: OwnedObjectPath,
        session_handle: OwnedObjectPath,
        app_id: String,
        parent_window: String,
        options: StartOptions,
    ) -> (response: u32, results: StartResults);
    method OpenPipeWireRemote(
        session_handle: OwnedObjectPath,
        options: OpenPipeWireRemoteOptions,
    ) -> (fd: OwnedFd);
    property version: u32, read;
    property AvailableSourceTypes: u32, read;
    property AvailableCursorModes: u32, read;
);

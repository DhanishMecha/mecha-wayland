use dbus::dbus_interface;
use std::collections::HashMap;
use zbus::zvariant::OwnedValue;

pub const NOTIFICATION_IFACE: &str = "org.freedesktop.impl.portal.Notification";
pub const NOTIFICATION_VERSION: u32 = 2;

dbus_interface!(pub NotificationIface = NOTIFICATION_IFACE;
    method AddNotification(app_id: String, id: String, notification: HashMap<String, OwnedValue>) -> ();
    method RemoveNotification(app_id: String, id: String) -> ();
    signal ActionInvoked(app_id: String, id: String, action: String, parameter: Vec<OwnedValue>);
    property version: u32, read;
    property SupportedOptions: HashMap<String, OwnedValue>, read;
);

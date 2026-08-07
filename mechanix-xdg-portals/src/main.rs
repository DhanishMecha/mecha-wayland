use access::{AccessBackend, access_module};
use account::{AccountBackend, account_module};
use app::RegisteredModule;
use app::prelude::*;
use app_chooser::{AppChooserBackend, app_chooser_module};
use background::{BackgroundBackend, background_module, backend::BackgroundIface};
use bluetooth::{BluetoothBackend, bluetooth_module};
use dbus::{DbusConnection, SessionBus, SystemBus, module as dbus_module};
use email::{EmailBackend, email_module};
use file_chooser::{FileChooserBackend, filechooser_module};
use inhibit::{InhibitBackend, inhibit_module};
use notification::{NotificationBackend, notification_module};
use polkit_agent::{AuthenticationAgentIface, PolkitAgentBackend, polkit_agent_module};
use portal_core::{DbusMonitor, PortalHost, dbus_monitor_module, portal_host_module};
use screencast::{ScreenCastBackend, screencast_module};
use screenshot::{ScreenshotBackend, screenshot_module};
use secret::{SecretBackend, secret_module};
use settings::{SettingsBackend, settings_module};

use io_ring::{Ring, RingSettings};
use window_manager::WindowManager;

#[derive(State)]
pub struct AppRoot {
    ring: Ring,
    dbus_system: DbusConnection<SystemBus>,
    dbus_session: DbusConnection<SessionBus>,
    system_monitor: DbusMonitor<SystemBus>,
    session_monitor: DbusMonitor<SessionBus>,
    window_manager: WindowManager,
    portal_host: PortalHost,
    backend: FileChooserBackend,
    bt_backend: BluetoothBackend,
    access_backend: AccessBackend,
    account_backend: AccountBackend,
    app_chooser_backend: AppChooserBackend,
    screenshot_backend: ScreenshotBackend,
    screencast_backend: ScreenCastBackend,
    settings_backend: SettingsBackend,
    notification_backend: NotificationBackend,
    secret_backend: SecretBackend,
    email_backend: EmailBackend,
    inhibit_backend: InhibitBackend,
    polkit_agent_backend: PolkitAgentBackend,
    background_backend: BackgroundBackend,
}

pub fn main_poll_module<S>() -> impl RegisteredModule<AppRoot, S> {
    Module::<AppRoot, _, _>::new().on(|s: &mut AppRoot, _: &app::Start| {
        s.window_manager.upload_atlas(&portal_core::atlas::UI);
        println!("[main] Service started. Waiting for portal D-Bus requests...");
    })
}

fn main() {
    let ring = Ring::new(RingSettings::default());
    let dbus_system = DbusConnection::<SystemBus>::new(ring.proxy());
    let dbus_session = DbusConnection::<SessionBus>::new(ring.proxy());
    let system_monitor = DbusMonitor::new(dbus_system.proxy());
    let session_monitor = DbusMonitor::new(dbus_session.proxy());
    let window_manager = WindowManager::new(ring.proxy());
    // One shared proxy for all session-bus portals — one name registration
    let session_proxy = dbus_session.proxy();
    let combined_xml = format!(
        "{}{}{}{}{}{}{}{}{}{}{}{}{}",
        file_chooser::backend::FileChooser::introspect(),
        access::backend::Access::introspect(),
        app_chooser::backend::AppChooserIface::introspect(),
        screenshot::backend::ScreenshotIface::introspect(),
        screencast::backend::ScreenCastIface::introspect(),
        settings::backend::SettingsIface::introspect(),
        notification::backend::NotificationIface::introspect(),
        secret::backend::SecretIface::introspect(),
        email::backend::EmailIface::introspect(),
        inhibit::backend::InhibitIface::introspect(),
        account::backend::AccountIface::introspect(),
        AuthenticationAgentIface::introspect(),
        BackgroundIface::introspect(),
    );
    let portal_host = PortalHost::new(session_proxy.clone(), combined_xml);
    let backend = FileChooserBackend::new(session_proxy.clone());
    let access_backend = AccessBackend::new(session_proxy.clone());
    let account_backend = AccountBackend::new(session_proxy.clone(), dbus_system.proxy());
    let app_chooser_backend = AppChooserBackend::new(session_proxy.clone());
    let screenshot_backend = ScreenshotBackend::new(session_proxy.clone());
    let screencast_backend = ScreenCastBackend::new(session_proxy.clone());
    let settings_backend = SettingsBackend::new(session_proxy.clone(), ring.proxy());
    let notification_backend = NotificationBackend::new(session_proxy.clone());
    let secret_backend = SecretBackend::new(session_proxy.clone());
    let email_backend = EmailBackend::new(session_proxy.clone());
    let inhibit_backend = InhibitBackend::new(session_proxy.clone(), dbus_system.proxy());
    let polkit_agent_backend = PolkitAgentBackend::new(session_proxy.clone(), dbus_system.proxy());
    let bt_backend = BluetoothBackend::new(dbus_system.proxy());
    let background_backend = BackgroundBackend::new(session_proxy.clone());

    let app_root = AppRoot {
        ring,
        dbus_system,
        dbus_session,
        system_monitor,
        session_monitor,
        window_manager,
        portal_host,
        backend,
        bt_backend,
        access_backend,
        account_backend,
        app_chooser_backend,
        screenshot_backend,
        screencast_backend,
        settings_backend,
        notification_backend,
        secret_backend,
        email_backend,
        inhibit_backend,
        polkit_agent_backend,
        background_backend,
    };

    let mut app = App::new(app_root)
        .mount(io_ring::module())
        .mount(main_poll_module())
        .mount(dbus_module::<SystemBus, _>())
        .mount(dbus_module::<SessionBus, _>())
        .mount(dbus_monitor_module::<SystemBus, _>())
        .mount(dbus_monitor_module::<SessionBus, _>())
        .mount(window_manager::module())
        .mount(portal_host_module())
        .mount(filechooser_module())
        .mount(bluetooth_module())
        .mount(access_module())
        .mount(account_module())
        .mount(app_chooser_module())
        .mount(screenshot_module())
        .mount(screencast_module())
        .mount(settings_module())
        .mount(notification_module())
        .mount(secret_module())
        .mount(email_module())
        .mount(inhibit_module())
        .mount(polkit_agent_module())
        .mount(background_module());

    println!("[main] Starting application event loop.");
    app.dispatch(&app::Start);
    loop {
        app.dispatch(&app::PrePoll);
        app.dispatch(&app::Poll);
    }
}

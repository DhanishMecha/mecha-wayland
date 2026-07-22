use access::{access_module, AccessBackend};
use app::prelude::*;
use app::RegisteredModule;
use bluetooth::{bluetooth_module, BluetoothBackend};
use dbus::{module as dbus_module, DbusConnection, SessionBus, SystemBus};
use file_chooser::{filechooser_module, FileChooserBackend};
use portal_core::{dbus_monitor_module, portal_host_module, DbusMonitor, PortalHost};
use screencast::{screencast_module, ScreenCastBackend};
use screenshot::{screenshot_module, ScreenshotBackend};

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
    screenshot_backend: ScreenshotBackend,
    screencast_backend: ScreenCastBackend,
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
        "{}{}{}{}",
        file_chooser::backend::FileChooser::introspect(),
        access::backend::Access::introspect(),
        screenshot::backend::ScreenshotIface::introspect(),
        screencast::backend::ScreenCastIface::introspect()
    );
    let portal_host = PortalHost::new(session_proxy.clone(), combined_xml);
    let backend = FileChooserBackend::new(session_proxy.clone());
    let access_backend = AccessBackend::new(session_proxy.clone());
    let screenshot_backend = ScreenshotBackend::new(session_proxy.clone());
    let screencast_backend = ScreenCastBackend::new(session_proxy.clone());
    let bt_backend = BluetoothBackend::new(dbus_system.proxy());

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
        screenshot_backend,
        screencast_backend,
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
        .mount(screenshot_module())
        .mount(screencast_module());

    println!("[main] Starting application event loop.");
    app.dispatch(&app::Start);
    loop {
        app.dispatch(&app::PrePoll);
        app.dispatch(&app::Poll);
    }
}


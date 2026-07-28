pub mod backend;
pub mod dialog;

pub use backend::{NotificationBackend, NotificationIface};

use app::RegisteredModule;

pub fn notification_module<S>() -> impl app::RegisteredModule<S, S>
where
    S: app::Lens<NotificationBackend> + app::Lens<window_manager::WindowManager> + 'static,
{
    app::Module::<S, _, _, _>::new()
        .mount(backend::notification_backend_module::<S>().into_module())
        .mount(dialog::notification_ui_module::<S>().into_module())
}

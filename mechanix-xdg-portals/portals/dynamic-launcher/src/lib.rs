pub mod backend;
pub mod dialog;

pub use backend::{DynamicLauncherBackend, DynamicLauncherIface};
use app::RegisteredModule;

pub fn dynamic_launcher_module<S>() -> impl app::RegisteredModule<S, S>
where
    S: app::Lens<DynamicLauncherBackend> + app::Lens<window_manager::WindowManager> + 'static,
{
    app::Module::<S, _, _, _>::new()
        .mount(backend::dynamic_launcher_backend_module::<S>().into_module())
        .mount(dialog::dynamic_launcher_ui_module::<S>().into_module())
}

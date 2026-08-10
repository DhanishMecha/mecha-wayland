pub mod backend;
pub mod dialog;

pub use backend::{GlobalShortcutsBackend, GlobalShortcutsIface};
use app::RegisteredModule;

pub fn global_shortcuts_module<S>() -> impl app::RegisteredModule<S, S>
where
    S: app::Lens<GlobalShortcutsBackend> + app::Lens<window_manager::WindowManager> + 'static,
{
    app::Module::<S, _, _, _>::new()
        .mount(backend::global_shortcuts_backend_module::<S>().into_module())
        .mount(dialog::global_shortcuts_ui_module::<S>().into_module())
}

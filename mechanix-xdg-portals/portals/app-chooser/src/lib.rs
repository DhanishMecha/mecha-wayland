pub mod backend;
pub mod dialog;

pub use backend::AppChooserBackend;
use app::RegisteredModule;

pub fn app_chooser_module<S>() -> impl app::RegisteredModule<S, S>
where
    S: app::Lens<AppChooserBackend> + app::Lens<window_manager::WindowManager> + 'static,
{
    app::Module::<S, _, _, _>::new()
        .mount(backend::app_chooser_backend_module::<S>().into_module())
        .mount(dialog::app_chooser_ui_module::<S>().into_module())
}

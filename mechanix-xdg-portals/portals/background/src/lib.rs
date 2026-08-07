pub mod backend;
pub mod dialog;

pub use backend::BackgroundBackend;
use app::RegisteredModule;

pub fn background_module<S>() -> impl app::RegisteredModule<S, S>
where
    S: app::Lens<BackgroundBackend> + app::Lens<window_manager::WindowManager> + 'static,
{
    app::Module::<S, _, _, _>::new()
        .mount(backend::background_backend_module::<S>().into_module())
        .mount(dialog::background_ui_module::<S>().into_module())
}

pub mod backend;
pub mod dialog;

pub use backend::AccessBackend;
use app::RegisteredModule;

pub fn access_module<S>() -> impl app::RegisteredModule<S, S>
where
    S: app::Lens<AccessBackend> + app::Lens<window_manager::WindowManager> + 'static,
{
    app::Module::<S, _, _, _>::new()
        .mount(backend::access_backend_module::<S>().into_module())
        .mount(dialog::access_ui_module::<S>().into_module())
}

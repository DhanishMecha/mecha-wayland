pub mod backend;
pub mod dialog;

pub use backend::AccountBackend;
use app::RegisteredModule;

pub fn account_module<S>() -> impl app::RegisteredModule<S, S>
where
    S: app::Lens<AccountBackend> + app::Lens<window_manager::WindowManager> + 'static,
{
    app::Module::<S, _, _, _>::new()
        .mount(backend::account_backend_module::<S>().into_module())
        .mount(dialog::account_ui_module::<S>().into_module())
}

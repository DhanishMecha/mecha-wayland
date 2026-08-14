use app::RegisteredModule;

pub mod backend;
pub mod dialog;
pub mod helpers;

pub use backend::{PrintBackend, PrintIface};

pub fn print_module<S>() -> impl app::RegisteredModule<S, S>
where
    S: app::Lens<PrintBackend> + app::Lens<window_manager::WindowManager> + 'static,
{
    app::Module::<S, _, _, _>::new()
        .mount(backend::print_backend_module::<S>().into_module())
        .mount(dialog::print_ui_module::<S>().into_module())
}

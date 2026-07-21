pub mod backend;
pub mod dialog;

pub use backend::FileChooserBackend;
use app::RegisteredModule;

pub fn filechooser_module<S>() -> impl app::RegisteredModule<S, S>
    where S: app::Lens<FileChooserBackend> + app::Lens<window_manager::WindowManager> + 'static
{
    app::Module::<S, _, _, _>
        ::new()
        .mount(backend::filechooser_module::<S>().into_module())
        .mount(dialog::filechooser_ui_module::<S>().into_module())
}

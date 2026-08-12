pub mod backend;
pub mod dialog;

pub use backend::{InputCaptureBackend, InputCaptureIface};
use app::RegisteredModule;

pub fn input_capture_module<S>() -> impl app::RegisteredModule<S, S>
where
    S: app::Lens<InputCaptureBackend> + app::Lens<window_manager::WindowManager> + 'static,
{
    app::Module::<S, _, _, _>::new()
        .mount(backend::input_capture_backend_module::<S>().into_module())
        .mount(dialog::input_capture_ui_module::<S>().into_module())
}

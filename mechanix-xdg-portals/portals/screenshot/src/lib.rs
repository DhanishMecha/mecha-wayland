pub mod backend;
pub mod dialog;

pub use backend::ScreenshotBackend;
use app::RegisteredModule;

pub fn screenshot_module<S>() -> impl app::RegisteredModule<S, S>
where
    S: app::Lens<ScreenshotBackend> + app::Lens<window_manager::WindowManager> + 'static,
{
    app::Module::<S, _, _, _>::new()
        .mount(backend::screenshot_backend_module::<S>().into_module())
        .mount(dialog::screenshot_ui_module::<S>().into_module())
}

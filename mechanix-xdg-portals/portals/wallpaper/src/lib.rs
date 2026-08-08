pub mod backend;
pub mod dialog;

pub use backend::WallpaperBackend;
use app::RegisteredModule;

pub fn wallpaper_module<S>() -> impl app::RegisteredModule<S, S>
where
    S: app::Lens<WallpaperBackend> + app::Lens<window_manager::WindowManager> + 'static,
{
    app::Module::<S, _, _, _>::new()
        .mount(backend::wallpaper_backend_module::<S>().into_module())
        .mount(dialog::wallpaper_ui_module::<S>().into_module())
}

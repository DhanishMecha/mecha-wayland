use app::RegisteredModule;

pub mod backend;
pub mod dialog;

pub use backend::ScreenCastBackend;

pub fn screencast_module<S>() -> impl app::RegisteredModule<S, S>
where
    S: app::Lens<ScreenCastBackend> + app::Lens<window_manager::WindowManager> + 'static,
{
    app::Module::<S, _, _, _>::new()
        .mount(backend::screencast_backend_module::<S>().into_module())
        .mount(dialog::screencast_ui_module::<S>().into_module())
}


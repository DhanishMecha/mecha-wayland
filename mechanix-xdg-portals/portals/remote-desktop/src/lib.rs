use app::RegisteredModule;

pub mod backend;
pub mod dialog;

pub use backend::RemoteDesktopBackend;

pub fn remote_desktop_module<S>() -> impl app::RegisteredModule<S, S>
where
    S: app::Lens<RemoteDesktopBackend> + app::Lens<window_manager::WindowManager> + 'static,
{
    app::Module::<S, _, _, _>::new()
        .mount(backend::remote_desktop_backend_module::<S>().into_module())
        .mount(dialog::remote_desktop_ui_module::<S>().into_module())
}

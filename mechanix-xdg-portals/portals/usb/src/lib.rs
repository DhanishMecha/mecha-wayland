pub mod backend;
pub mod dialog;

pub use backend::{UsbBackend, UsbIface};
use app::RegisteredModule;

pub fn usb_module<S>() -> impl app::RegisteredModule<S, S>
where
    S: app::Lens<UsbBackend> + app::Lens<window_manager::WindowManager> + 'static,
{
    app::Module::<S, _, _, _>::new()
        .mount(backend::usb_backend_module::<S>().into_module())
        .mount(dialog::usb_ui_module::<S>().into_module())
}

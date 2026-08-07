pub mod backend;

pub use backend::{ClipboardBackend, ClipboardIface};

use app::RegisteredModule;

/// Creates and mounts the clipboard portal backend module.
pub fn clipboard_module<S>() -> impl app::RegisteredModule<S, S>
where
    S: app::Lens<ClipboardBackend> + 'static,
{
    app::Module::<S, _, _>::new()
        .mount(backend::clipboard_backend_module::<S>().into_module())
}

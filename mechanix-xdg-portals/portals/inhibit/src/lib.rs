pub mod backend;

pub use backend::{InhibitBackend, InhibitIface};

use app::RegisteredModule;

pub fn inhibit_module<S>() -> impl app::RegisteredModule<S, S>
where
    S: app::Lens<InhibitBackend> + 'static,
{
    app::Module::<S, _, _, _>::new()
        .mount(backend::inhibit_backend_module::<S>().into_module())
}

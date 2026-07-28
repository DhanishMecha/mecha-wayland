use app::RegisteredModule;

pub mod backend;

pub use backend::{SecretBackend, SecretIface};

pub fn secret_module<S>() -> impl app::RegisteredModule<S, S>
where
    S: app::Lens<SecretBackend> + 'static,
{
    app::Module::<S, _, _, _>::new().mount(backend::secret_backend_module::<S>().into_module())
}

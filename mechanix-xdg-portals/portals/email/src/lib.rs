use app::RegisteredModule;

pub mod backend;

pub use backend::{EmailBackend, EmailIface};

pub fn email_module<S>() -> impl app::RegisteredModule<S, S>
where
    S: app::Lens<EmailBackend> + 'static,
{
    app::Module::<S, _, _, _>::new().mount(backend::email_backend_module::<S>().into_module())
}

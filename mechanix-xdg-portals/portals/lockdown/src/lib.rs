use app::RegisteredModule;

pub mod backend;

pub use backend::LockdownBackend;

pub fn lockdown_module<S>() -> impl app::RegisteredModule<S, S>
where
    S: app::Lens<LockdownBackend> + 'static,
{
    app::Module::<S, _, _, _>::new()
        .mount(backend::lockdown_backend_module::<S>().into_module())
}

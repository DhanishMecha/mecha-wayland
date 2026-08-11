use app::RegisteredModule;

pub mod backend;

pub use backend::{PermissionStoreBackend, PermissionStoreIface};

pub fn permission_store_module<S>() -> impl RegisteredModule<S, S>
where
    S: app::Lens<PermissionStoreBackend> + 'static,
{
    app::Module::<S, _, _, _>::new().mount(backend::permission_store_backend_module::<S>().into_module())
}

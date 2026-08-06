pub mod backend;
pub mod dialog;

pub use backend::PolkitAgentBackend;
pub use backend::AuthenticationAgentIface;

use app::RegisteredModule;

pub fn polkit_agent_module<S>() -> impl app::RegisteredModule<S, S>
where
    S: app::Lens<PolkitAgentBackend>
        + app::Lens<window_manager::WindowManager>
        + 'static,
{
    app::Module::<S, _, _, _>::new()
        .mount(backend::polkit_agent_backend_module::<S>().into_module())
        .mount(dialog::polkit_dialog_module::<S>().into_module())
}

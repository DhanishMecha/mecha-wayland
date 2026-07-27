use app::RegisteredModule;

pub mod backend;

pub use backend::{ SettingsBackend, SettingsIface };
pub use backend::{
    AccentColor, ColorScheme, Contrast, ReducedMotion,
    APPEARANCE_NS, KEY_ACCENT_COLOR, KEY_COLOR_SCHEME, KEY_CONTRAST, KEY_REDUCED_MOTION,
};

pub fn settings_module<S>() -> impl app::RegisteredModule<S, S>
    where S: app::Lens<SettingsBackend> + 'static
{
    app::Module::<S, _, _, _>::new().mount(backend::settings_backend_module::<S>().into_module())
}

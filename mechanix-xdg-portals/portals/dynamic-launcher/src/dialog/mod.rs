mod dialog;

pub use dialog::DynamicLauncherDialogUi;

use crate::backend::{DynamicLauncherRequest, DynamicLauncherResponse, RequestHandle};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use window_manager::{WindowId, WindowKind, WindowManager, WindowSettings};

pub fn dynamic_launcher_ui_module<S>() -> impl app::RegisteredModule<WindowManager, S>
where
    S: app::Lens<WindowManager> + 'static,
{
    let active_windows = Rc::new(RefCell::new(HashMap::<RequestHandle, WindowId>::new()));

    app::Module::<WindowManager, _, _>::new()
        .mount(ui::register_events!(DynamicLauncherResponse))
        .on({
            let active_windows = active_windows.clone();
            move |wm: &mut WindowManager, cmd: &DynamicLauncherRequest| match cmd {
                DynamicLauncherRequest::PrepareInstall(info) => {
                    println!(
                        "[dynamic-launcher-ui] Spawning consent dialog for app={} (handle={})",
                        info.app_id, info.handle
                    );

                    let id = wm.spawn_window(
                        WindowSettings {
                            width: 500,
                            height: 420,
                            clear_color: window_manager::Color::rgb(0.08, 0.08, 0.1),
                            kind: WindowKind::Xdg {
                                title: "Create Application Launcher".to_string(),
                            },
                            touch_config: None,
                            gesture_config: None,
                        },
                        DynamicLauncherDialogUi::new(
                            info.handle.clone(),
                            info.app_id.clone(),
                            info.name.clone(),
                            info.launcher_type,
                            info.target.clone(),
                        ),
                    );

                    active_windows.borrow_mut().insert(info.handle.clone(), id);
                    wm.flush_pending();
                }
                DynamicLauncherRequest::Close { handle } => {
                    let id = active_windows.borrow_mut().remove(handle);
                    if let Some(id) = id {
                        wm.destroy(id);
                    }
                }
            }
        })
        .on(move |wm: &mut WindowManager, resp: &DynamicLauncherResponse| {
            let id = active_windows.borrow_mut().remove(&resp.handle);
            if let Some(id) = id {
                wm.destroy(id);
            }
        })
}

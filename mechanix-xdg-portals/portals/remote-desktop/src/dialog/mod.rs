mod dialog;

pub use dialog::RemoteDesktopDialogUi;

use crate::backend::{RemoteDesktopRequest, RemoteDesktopResponse, RequestHandle};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use window_manager::{WindowId, WindowKind, WindowManager, WindowSettings};

pub fn remote_desktop_ui_module<S>() -> impl app::RegisteredModule<WindowManager, S>
where
    S: app::Lens<WindowManager> + 'static,
{
    let active_windows = Rc::new(RefCell::new(HashMap::<RequestHandle, WindowId>::new()));

    app::Module::<WindowManager, _, _>::new()
        .mount(ui::register_events!(RemoteDesktopResponse))
        .on({
            let active_windows = active_windows.clone();
            move |wm: &mut WindowManager, cmd: &RemoteDesktopRequest| {
                println!(
                    "[remote-desktop-ui] Spawning Start dialog for \
                     app={} handle={} device_types={}",
                    cmd.app_id, cmd.handle, cmd.device_types
                );

                // TODO: implement a proper RemoteDesktopDialogUi widget
                let id = wm.spawn_window(
                    WindowSettings {
                        width: 540,
                        height: 480,
                        clear_color: window_manager::Color::rgb(0.08, 0.08, 0.1),
                        kind: WindowKind::Xdg {
                            title: "Remote Desktop - Allow Access".to_string(),
                        },
                        touch_config: None,
                        gesture_config: None,
                    },
                    RemoteDesktopDialogUi::new(
                        cmd.handle.clone(),
                        cmd.app_id.clone(),
                        cmd.device_types,
                    ),
                );

                active_windows.borrow_mut().insert(cmd.handle.clone(), id);
                wm.flush_pending();
            }
        })
        .on(move |wm: &mut WindowManager, resp: &RemoteDesktopResponse| {
            let id = active_windows.borrow_mut().remove(&resp.handle);
            if let Some(id) = id {
                println!(
                    "[remote-desktop-ui] Dialog completed ({:?}). Closing window.",
                    resp.outcome
                );
                wm.destroy(id);
            }
        })
}

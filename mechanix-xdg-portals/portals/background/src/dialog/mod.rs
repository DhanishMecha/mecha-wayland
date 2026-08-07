mod dialog;

pub use dialog::BackgroundDialogUi;

use crate::backend::{BackgroundRequest, BackgroundResponse, RequestHandle};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use window_manager::{WindowId, WindowKind, WindowManager, WindowSettings};

pub fn background_ui_module<S>() -> impl app::RegisteredModule<WindowManager, S>
where
    S: app::Lens<WindowManager> + 'static,
{
    let active_windows = Rc::new(RefCell::new(HashMap::<RequestHandle, WindowId>::new()));

    app::Module::<WindowManager, _, _>::new()
        .mount(ui::register_events!(BackgroundResponse))
        .on({
            let active_windows = active_windows.clone();
            move |wm: &mut WindowManager, cmd: &BackgroundRequest| match cmd {
                BackgroundRequest::NotifyBackground {
                    handle,
                    app_id,
                    name,
                } => {
                    println!(
                        "[background-ui] Spawning Background dialog for app={app_id} (handle={handle})."
                    );

                    let id = wm.spawn_window(
                        WindowSettings {
                            width: 480,
                            height: 300,
                            clear_color: window_manager::Color::rgb(0.08, 0.08, 0.1),
                            kind: WindowKind::Xdg {
                                title: format!("Background Activity: {name}"),
                            },
                            touch_config: None,
                            gesture_config: None,
                        },
                        BackgroundDialogUi::new(
                            handle.clone(),
                            app_id.clone(),
                            name.clone(),
                        ),
                    );

                    active_windows.borrow_mut().insert(handle.clone(), id);
                    wm.flush_pending();
                }
                BackgroundRequest::Close { handle } => {
                    println!("[background-ui] Portal requested close for handle={handle}.");
                    let id = active_windows.borrow_mut().remove(handle);
                    if let Some(id) = id {
                        wm.destroy(id);
                    }
                }
            }
        })
        .on(move |wm: &mut WindowManager, resp: &BackgroundResponse| {
            let id = active_windows.borrow_mut().remove(&resp.handle);
            if let Some(id) = id {
                println!(
                    "[background-ui] Dialog completed ({:?}). Closing window.",
                    resp.outcome
                );
                wm.destroy(id);
            }
        })
}

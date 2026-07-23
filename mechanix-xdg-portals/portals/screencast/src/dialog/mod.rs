mod dialog;

pub use dialog::ScreenCastDialogUi;

use crate::backend::{RequestHandle, ScreenCastRequest, ScreenCastResponse};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use window_manager::{WindowId, WindowKind, WindowManager, WindowSettings};

pub fn screencast_ui_module<S>() -> impl app::RegisteredModule<WindowManager, S>
where
    S: app::Lens<WindowManager> + 'static,
{
    let active_windows = Rc::new(RefCell::new(HashMap::<RequestHandle, WindowId>::new()));

    app::Module::<WindowManager, _, _>::new()
        .mount(ui::register_events!(ScreenCastResponse))
        .on({
            let active_windows = active_windows.clone();
            move |wm: &mut WindowManager, cmd: &ScreenCastRequest| match cmd {
                ScreenCastRequest::Start {
                    handle,
                    session_handle: _,
                    app_id,
                    types,
                    cursor_mode,
                    multiple,
                } => {
                    println!(
                        "[screencast-ui] Spawning Start dialog for app={app_id} handle={handle} \
                         types={types} cursor_mode={cursor_mode:?} multiple={multiple}."
                    );

                    let id = wm.spawn_window(
                        WindowSettings {
                            width: 540,
                            height: 620,
                            clear_color: window_manager::Color::rgb(0.08, 0.08, 0.1),
                            kind: WindowKind::Xdg {
                                title: "Screencast - Start Sharing".to_string(),
                            },
                            touch_config: None,
                            gesture_config: None,
                        },
                        ScreenCastDialogUi::new(
                            handle.clone(),
                            app_id.clone(),
                            "Start Screencast".to_string(),
                            "Allow this application to start streaming your screen?".to_string(),
                            "🎥",
                            "Cancel",
                            "Start Stream",
                            *types,
                        ),
                    );

                    active_windows.borrow_mut().insert(handle.clone(), id);
                    wm.flush_pending();
                }

                ScreenCastRequest::Close { handle } => {
                    println!("[screencast-ui] Portal requested close for handle={handle}.");
                    let id = active_windows.borrow_mut().remove(handle);
                    if let Some(id) = id {
                        wm.destroy(id);
                    }
                }
            }
        })
        .on(move |wm: &mut WindowManager, resp: &ScreenCastResponse| {
            let id = active_windows.borrow_mut().remove(&resp.handle);
            if let Some(id) = id {
                println!(
                    "[screencast-ui] Dialog completed ({:?}). Closing window.",
                    resp.outcome
                );
                wm.destroy(id);
            }
        })
}

mod dialog;

pub use dialog::ScreenshotDialogUi;

use crate::backend::{RequestHandle, ScreenshotRequest, ScreenshotResponse};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use window_manager::{WindowId, WindowKind, WindowManager, WindowSettings};

pub fn screenshot_ui_module<S>() -> impl app::RegisteredModule<WindowManager, S>
where
    S: app::Lens<WindowManager> + 'static,
{
    let active_windows = Rc::new(RefCell::new(HashMap::<RequestHandle, WindowId>::new()));

    app::Module::<WindowManager, _, _>::new()
        .mount(ui::register_events!(ScreenshotResponse))
        .on({
            let active_windows = active_windows.clone();
            move |wm: &mut WindowManager, cmd: &ScreenshotRequest| match cmd {
                ScreenshotRequest::Screenshot {
                    handle,
                    app_id,
                    interactive,
                    target,
                } => {
                    println!(
                        "[screenshot-ui] Spawning Screenshot dialog for app={app_id} \
                         handle={handle} interactive={interactive} target={target:?}."
                    );

                    let body = match target {
                        Some(1) => "This app wants to take a screenshot of your entire screen.",
                        Some(2) => "This app wants to capture a window of your screen.",
                        Some(4) => "This app wants to capture an area of your screen.",
                        Some(8) => "This app wants to capture the active window.",
                        _ if *interactive => {
                            "This app wants to capture a region or window of your screen."
                        }
                        _ => "This app wants to take a screenshot of your entire screen.",
                    };

                    let id = wm.spawn_window(
                        WindowSettings {
                            width: 540,
                            height: 620,
                            clear_color: window_manager::Color::rgb(0.08, 0.08, 0.1),
                            kind: WindowKind::Xdg {
                                title: "Screenshot".to_string(),
                            },
                            touch_config: None,
                            gesture_config: None,
                        },
                        ScreenshotDialogUi::new(
                            handle.clone(),
                            app_id.clone(),
                            "Screenshot".to_string(),
                            body.to_string(),
                            "📷",
                            "Cancel",
                            "Take Screenshot",
                        ),
                    );

                    active_windows.borrow_mut().insert(handle.clone(), id);
                    wm.flush_pending();
                }

                ScreenshotRequest::PickColor { handle, app_id } => {
                    println!(
                        "[screenshot-ui] Spawning PickColor dialog for app={app_id} \
                         handle={handle}."
                    );

                    let id = wm.spawn_window(
                        WindowSettings {
                            width: 540,
                            height: 620,
                            clear_color: window_manager::Color::rgb(0.08, 0.08, 0.1),
                            kind: WindowKind::Xdg {
                                title: "Pick Color".to_string(),
                            },
                            touch_config: None,
                            gesture_config: None,
                        },
                        ScreenshotDialogUi::new(
                            handle.clone(),
                            app_id.clone(),
                            "Pick Color".to_string(),
                            "This app wants to pick a colour from your screen.".to_string(),
                            "🎨",
                            "Cancel",
                            "Pick Color",
                        ),
                    );

                    active_windows.borrow_mut().insert(handle.clone(), id);
                    wm.flush_pending();
                }

                ScreenshotRequest::Close { handle } => {
                    println!("[screenshot-ui] Portal requested close for handle={handle}.");
                    let id = active_windows.borrow_mut().remove(handle);
                    if let Some(id) = id {
                        wm.destroy(id);
                    }
                }
            }
        })
        .on(move |wm: &mut WindowManager, resp: &ScreenshotResponse| {
            let id = active_windows.borrow_mut().remove(&resp.handle);
            if let Some(id) = id {
                println!(
                    "[screenshot-ui] Dialog completed ({:?}). Closing window.",
                    resp.outcome
                );
                wm.destroy(id);
            }
        })
}

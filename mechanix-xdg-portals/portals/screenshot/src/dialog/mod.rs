mod dialog;

pub use dialog::ScreenshotDialogUi;

use crate::backend::{RequestHandle, ScreenshotRequest, ScreenshotResponse};
use std::cell::Cell;
use std::collections::HashMap;
use window_manager::{WindowId, WindowKind, WindowManager, WindowSettings};

thread_local! {
    pub static PENDING_DIALOG: Cell<Option<ScreenshotResponse>> = const { Cell::new(None) };
    pub static ACTIVE_WINDOWS: std::cell::RefCell<
        HashMap<RequestHandle, WindowId>
    > = std::cell::RefCell::new(HashMap::new());
}

pub fn screenshot_ui_module<S>() -> impl app::RegisteredModule<WindowManager, S>
where
    S: app::Lens<WindowManager> + 'static,
{
    app::Module::<WindowManager, _, _>::new()
        .on(
            |wm: &mut WindowManager, cmd: &ScreenshotRequest| match cmd {
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

                    ACTIVE_WINDOWS.with(|wins| {
                        wins.borrow_mut().insert(handle.clone(), id);
                    });
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

                    ACTIVE_WINDOWS.with(|wins| {
                        wins.borrow_mut().insert(handle.clone(), id);
                    });
                    wm.flush_pending();
                }

                ScreenshotRequest::Close { handle } => {
                    println!("[screenshot-ui] Portal requested close for handle={handle}.");
                    let id = ACTIVE_WINDOWS.with(|wins| wins.borrow_mut().remove(handle));
                    if let Some(id) = id {
                        wm.destroy(id);
                    }
                }
            },
        )
        .on(
            |wm: &mut WindowManager, _: &app::Poll| -> Option<ScreenshotResponse> {
                let done = PENDING_DIALOG.take();
                if let Some(ref done) = done {
                    let id = ACTIVE_WINDOWS.with(|wins| wins.borrow_mut().remove(&done.handle));
                    if let Some(id) = id {
                        println!(
                            "[screenshot-ui] Dialog completed ({:?}). Closing window.",
                            done.outcome
                        );
                        wm.destroy(id);
                    }
                }
                done
            },
        )
}

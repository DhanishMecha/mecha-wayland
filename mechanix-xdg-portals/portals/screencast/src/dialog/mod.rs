mod dialog;

pub use dialog::ScreenCastDialogUi;

use crate::backend::{RequestHandle, ScreenCastRequest, ScreenCastResponse};
use std::cell::Cell;
use std::collections::HashMap;
use window_manager::{WindowId, WindowKind, WindowManager, WindowSettings};

thread_local! {
    pub static PENDING_DIALOG: Cell<Option<ScreenCastResponse>> = const { Cell::new(None) };
    pub static ACTIVE_WINDOWS: std::cell::RefCell<
        HashMap<RequestHandle, WindowId>
    > = std::cell::RefCell::new(HashMap::new());
}

pub fn screencast_ui_module<S>() -> impl app::RegisteredModule<WindowManager, S>
where
    S: app::Lens<WindowManager> + 'static,
{
    app::Module::<WindowManager, _, _>::new()
        .on(
            |wm: &mut WindowManager, cmd: &ScreenCastRequest| match cmd {
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

                    ACTIVE_WINDOWS.with(|wins| {
                        wins.borrow_mut().insert(handle.clone(), id);
                    });
                    wm.flush_pending();
                }

                ScreenCastRequest::Close { handle } => {
                    println!("[screencast-ui] Portal requested close for handle={handle}.");
                    let id = ACTIVE_WINDOWS.with(|wins| wins.borrow_mut().remove(handle));
                    if let Some(id) = id {
                        wm.destroy(id);
                    }
                }
            },
        )
        .on(
            |wm: &mut WindowManager, _: &app::Poll| -> Option<ScreenCastResponse> {
                let done = PENDING_DIALOG.take();
                if let Some(ref done) = done {
                    let id = ACTIVE_WINDOWS.with(|wins| wins.borrow_mut().remove(&done.handle));
                    if let Some(id) = id {
                        println!(
                            "[screencast-ui] Dialog completed ({:?}). Closing window.",
                            done.outcome
                        );
                        wm.destroy(id);
                    }
                }
                done
            },
        )
}

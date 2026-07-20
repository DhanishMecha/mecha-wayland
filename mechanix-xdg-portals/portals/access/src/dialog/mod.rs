mod dialog;

pub use dialog::AccessDialogUi;

use crate::backend::{AccessRequest, AccessResponse, RequestHandle};
use std::cell::Cell;
use std::collections::HashMap;
use window_manager::{WindowId, WindowKind, WindowManager, WindowSettings};

// Thread-local slots for passing results back to the app main loop.
thread_local! {
    pub static PENDING_DIALOG: Cell<Option<AccessResponse>> = const { Cell::new(None) };
    pub static ACTIVE_WINDOWS: std::cell::RefCell<HashMap<RequestHandle, WindowId>> =
        std::cell::RefCell::new(HashMap::new());
}

/// Coordinator: spawns / closes Access dialog windows and polls for results.
pub fn access_ui_module<S>() -> impl app::RegisteredModule<WindowManager, S>
where
    S: app::Lens<WindowManager> + 'static,
{
    app::Module::<WindowManager, _, _>::new()
        .on(|wm: &mut WindowManager, cmd: &AccessRequest| {
            match cmd {
                AccessRequest::AccessDialog {
                    handle,
                    app_id,
                    title,
                    subtitle,
                    body,
                    options,
                } => {
                    println!(
                        "[access-ui] Spawning Access dialog for app={app_id} (handle={handle})."
                    );

                    let icon = options.icon.clone();
                    let deny_label = options.deny_label.clone();
                    let grant_label = options.grant_label.clone();
                    let choices = options.choices.clone();

                    // TODO: Pass parent_window identifier and respect options.modal for window parenting
                    let id = wm.spawn_window(
                        WindowSettings {
                            width: 540,
                            height: 620,
                            clear_color: window_manager::Color::rgb(0.08, 0.08, 0.1),
                            kind: WindowKind::Xdg {
                                title: title.clone(),
                            },
                            touch_config: None,
                            gesture_config: None,
                        },
                        AccessDialogUi::new(
                            handle.clone(),
                            app_id.clone(),
                            title.clone(),
                            subtitle.clone(),
                            body.clone(),
                            icon,
                            deny_label,
                            grant_label,
                            choices,
                        ),
                    );

                    ACTIVE_WINDOWS.with(|wins| {
                        wins.borrow_mut().insert(handle.clone(), id);
                    });
                    wm.flush_pending();
                }
                AccessRequest::Close { handle } => {
                    println!("[access-ui] Portal requested close for handle={handle}.");
                    let id = ACTIVE_WINDOWS.with(|wins| wins.borrow_mut().remove(handle));
                    if let Some(id) = id {
                        wm.destroy(id);
                    }
                }
            }
        })
        .on(
            |wm: &mut WindowManager, _: &app::Poll| -> Option<AccessResponse> {
                let done = PENDING_DIALOG.take();
                if let Some(ref done) = done {
                    let id = ACTIVE_WINDOWS.with(|wins| wins.borrow_mut().remove(&done.handle));
                    if let Some(id) = id {
                        println!(
                            "[access-ui] Dialog completed ({:?}). Closing window.",
                            done.outcome
                        );
                        wm.destroy(id);
                    }
                }
                done
            },
        )
}

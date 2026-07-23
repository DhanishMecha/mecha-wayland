mod dialog;

pub use dialog::AccessDialogUi;

use crate::backend::{AccessRequest, AccessResponse, RequestHandle};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use window_manager::{WindowId, WindowKind, WindowManager, WindowSettings};

/// Coordinator: spawns / closes Access dialog windows and polls for results.
pub fn access_ui_module<S>() -> impl app::RegisteredModule<WindowManager, S>
where
    S: app::Lens<WindowManager> + 'static,
{
    let active_windows = Rc::new(RefCell::new(HashMap::<RequestHandle, WindowId>::new()));

    app::Module::<WindowManager, _, _>::new()
        .mount(ui::register_events!(AccessResponse))
        .on({
            let active_windows = active_windows.clone();
            move |wm: &mut WindowManager, cmd: &AccessRequest| match cmd {
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

                    active_windows.borrow_mut().insert(handle.clone(), id);
                    wm.flush_pending();
                }
                AccessRequest::Close { handle } => {
                    println!("[access-ui] Portal requested close for handle={handle}.");
                    let id = active_windows.borrow_mut().remove(handle);
                    if let Some(id) = id {
                        wm.destroy(id);
                    }
                }
            }
        })
        .on(move |wm: &mut WindowManager, resp: &AccessResponse| {
            let id = active_windows.borrow_mut().remove(&resp.handle);
            if let Some(id) = id {
                println!(
                    "[access-ui] Dialog completed ({:?}). Closing window.",
                    resp.outcome
                );
                wm.destroy(id);
            }
        })
}

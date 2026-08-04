mod dialog;

pub use dialog::AccountDialogUi;

use crate::backend::{AccountRequest, AccountResponse, RequestHandle};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use window_manager::{WindowId, WindowKind, WindowManager, WindowSettings};

/// Coordinator: spawns / closes Account consent dialog windows.
pub fn account_ui_module<S>() -> impl app::RegisteredModule<WindowManager, S>
where
    S: app::Lens<WindowManager> + 'static,
{
    let active_windows = Rc::new(RefCell::new(HashMap::<RequestHandle, WindowId>::new()));

    app::Module::<WindowManager, _, _>::new()
        .mount(ui::register_events!(AccountResponse))
        .on({
            let active_windows = active_windows.clone();
            move |wm: &mut WindowManager, cmd: &AccountRequest| match cmd {
                AccountRequest::GetUserInformation {
                    handle,
                    app_id,
                    reason,
                    user_info,
                } => {
                    println!(
                        "[account-ui] Spawning consent dialog for app={app_id} (handle={handle})"
                    );

                    let id = wm.spawn_window(
                        WindowSettings {
                            width: 500,
                            height: 420,
                            clear_color: window_manager::Color::rgb(0.08, 0.08, 0.1),
                            kind: WindowKind::Xdg {
                                title: "Share Account Information".to_string(),
                            },
                            touch_config: None,
                            gesture_config: None,
                        },
                        AccountDialogUi::new(
                            handle.clone(),
                            app_id.clone(),
                            reason.clone(),
                            user_info.clone(),
                        ),
                    );

                    active_windows.borrow_mut().insert(handle.clone(), id);
                    wm.flush_pending();
                }
                AccountRequest::Close { handle } => {
                    println!("[account-ui] Portal requested close for handle={handle}");
                    let id = active_windows.borrow_mut().remove(handle);
                    if let Some(id) = id {
                        wm.destroy(id);
                    }
                }
            }
        })
        .on(move |wm: &mut WindowManager, resp: &AccountResponse| {
            let id = active_windows.borrow_mut().remove(&resp.handle);
            if let Some(id) = id {
                println!(
                    "[account-ui] Dialog completed ({:?}). Closing window.",
                    resp.outcome
                );
                wm.destroy(id);
            }
        })
}

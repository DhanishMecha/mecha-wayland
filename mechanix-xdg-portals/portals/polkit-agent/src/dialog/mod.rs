mod dialog;

pub use dialog::PolkitAuthDialog;

use crate::backend::{AuthCancelled, AuthRequest, AuthResponse};
use crate::backend::RequestHandle;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use window_manager::{WindowId, WindowKind, WindowManager, WindowSettings};

/// Coordinator: spawns / closes polkit auth dialog windows and routes results.
pub fn polkit_dialog_module<S>() -> impl app::RegisteredModule<WindowManager, S>
where
    S: app::Lens<WindowManager> + 'static,
{
    let active_windows = Rc::new(RefCell::new(HashMap::<RequestHandle, WindowId>::new()));

    app::Module::<WindowManager, _, _>::new()
        .mount(ui::register_events!(AuthResponse))
        // Open auth dialog when backend emits AuthRequest.
        .on({
            let active_windows = active_windows.clone();
            move |wm: &mut WindowManager, req: &AuthRequest| {
                println!(
                    "[polkit-dialog] Spawning auth dialog for action={} cookie={}",
                    req.action_id, req.cookie
                );

                let id = wm.spawn_window(
                    WindowSettings {
                        width: 480,
                        height: 340,
                        clear_color: window_manager::Color::rgb(0.08, 0.08, 0.1),
                        kind: WindowKind::Xdg {
                            title: "Authentication Required".to_string(),
                        },
                        touch_config: None,
                        gesture_config: None,
                    },
                    PolkitAuthDialog::new(
                        req.cookie.clone(),
                        req.action_id.clone(),
                        req.message.clone(),
                        req.icon_name.clone(),
                        &req.identities,
                    ),
                );

                println!(
                    "[polkit-dialog] Spawned auth window for cookie={}",
                    req.cookie
                );

                active_windows.borrow_mut().insert(req.cookie.clone(), id);
                wm.flush_pending();
            }
        })
        // Close dialog when polkitd sends CancelAuthentication.
        .on({
            let active_windows = active_windows.clone();
            move |wm: &mut WindowManager, cancelled: &AuthCancelled| {
                println!(
                    "[polkit-dialog] Closing dialog due to server cancel: cookie={}",
                    cancelled.cookie
                );
                let id = active_windows.borrow_mut().remove(&cancelled.cookie);
                if let Some(id) = id {
                    wm.destroy(id);
                }
            }
        })
        // Close dialog after user responds (success or cancel).
        .on(move |wm: &mut WindowManager, resp: &AuthResponse| {
            let id = active_windows.borrow_mut().remove(&resp.cookie);
            if let Some(id) = id {
                println!(
                    "[polkit-dialog] Dialog completed. Closing window for cookie={}",
                    resp.cookie
                );
                wm.destroy(id);
            }
        })
}

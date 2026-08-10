mod dialog;

pub use dialog::GlobalShortcutsDialogUi;

use crate::backend::types::{GlobalShortcutsRequest, GlobalShortcutsResponse};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use window_manager::{WindowId, WindowKind, WindowManager, WindowSettings};

pub fn global_shortcuts_ui_module<S>() -> impl app::RegisteredModule<WindowManager, S>
where
    S: app::Lens<WindowManager> + 'static,
{
    let active_windows = Rc::new(RefCell::new(HashMap::<String, WindowId>::new()));

    app::Module::<WindowManager, _, _>::new()
        .mount(ui::register_events!(GlobalShortcutsResponse))
        .on({
            let active_windows = active_windows.clone();
            move |wm: &mut WindowManager, cmd: &GlobalShortcutsRequest| match cmd {
                GlobalShortcutsRequest::BindDialog {
                    handle,
                    app_id,
                    shortcuts,
                } => {
                    println!(
                        "[global-shortcuts-ui] Spawning Bind dialog for app={app_id} handle={handle}."
                    );

                    let id = wm.spawn_window(
                        WindowSettings {
                            width: 500,
                            height: 380,
                            clear_color: window_manager::Color::rgb(0.08, 0.08, 0.1),
                            kind: WindowKind::Xdg {
                                title: "Global Shortcuts".to_string(),
                            },
                            touch_config: None,
                            gesture_config: None,
                        },
                        GlobalShortcutsDialogUi::new(
                            handle.clone(),
                            app_id.clone(),
                            "Global Shortcuts Request".to_string(),
                            shortcuts.clone(),
                            "⌨️",
                            "Deny",
                            "Allow",
                        ),
                    );

                    active_windows.borrow_mut().insert(handle.clone(), id);
                    wm.flush_pending();
                }

                GlobalShortcutsRequest::ConfigureDialog {
                    handle,
                    app_id,
                    shortcuts,
                } => {
                    println!(
                        "[global-shortcuts-ui] Spawning Configure dialog for app={app_id} handle={handle}."
                    );

                    let id = wm.spawn_window(
                        WindowSettings {
                            width: 500,
                            height: 380,
                            clear_color: window_manager::Color::rgb(0.08, 0.08, 0.1),
                            kind: WindowKind::Xdg {
                                title: "Configure Global Shortcuts".to_string(),
                            },
                            touch_config: None,
                            gesture_config: None,
                        },
                        GlobalShortcutsDialogUi::new(
                            handle.clone(),
                            app_id.clone(),
                            "Active Shortcuts".to_string(),
                            shortcuts.clone(),
                            "⚙️",
                            "Close",
                            "",
                        ),
                    );

                    active_windows.borrow_mut().insert(handle.clone(), id);
                    wm.flush_pending();
                }

                GlobalShortcutsRequest::Close { handle } => {
                    println!("[global-shortcuts-ui] Portal requested close for handle={handle}.");
                    let id = active_windows.borrow_mut().remove(handle);
                    if let Some(id) = id {
                        wm.destroy(id);
                    }
                }
            }
        })
        .on(move |wm: &mut WindowManager, resp: &GlobalShortcutsResponse| {
            let id = active_windows.borrow_mut().remove(&resp.handle);
            if let Some(id) = id {
                println!(
                    "[global-shortcuts-ui] Dialog completed ({:?}). Closing window.",
                    resp.outcome
                );
                wm.destroy(id);
            }
        })
}

mod dialog;
mod widgets;

pub use dialog::AppChooserDialogUi;

use crate::backend::{AppChooserRequest, AppChooserResponse, RequestHandle};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use window_manager::{WindowId, WindowKind, WindowManager, WindowSettings};

/// Coordinator: spawns / closes AppChooser dialog windows and handles results via event dispatch.
pub fn app_chooser_ui_module<S>() -> impl app::RegisteredModule<WindowManager, S>
where
    S: app::Lens<WindowManager> + 'static,
{
    let active_windows = Rc::new(RefCell::new(HashMap::<RequestHandle, WindowId>::new()));

    // Shared map of pending choice updates: handle -> new choices.
    // The coordinator inserts new choices; the dialog widget drains it on its next on_event() call.
    let pending_updates: Rc<RefCell<HashMap<RequestHandle, Vec<String>>>> =
        Rc::new(RefCell::new(HashMap::new()));

    app::Module::<WindowManager, _, _>::new()
        .mount(ui::register_events!(AppChooserResponse))
        .on({
            let active_windows = active_windows.clone();
            let pending_updates = pending_updates.clone();
            move |wm: &mut WindowManager, cmd: &AppChooserRequest| match cmd {
                AppChooserRequest::ChooseApplication {
                    handle,
                    app_id,
                    choices,
                    last_choice,
                    content_type,
                    uri,
                    filename,
                } => {
                    println!(
                        "[app-chooser-ui] Spawning picker for app={app_id} ({} choices, handle={handle}).",
                        choices.len()
                    );

                    let id = wm.spawn_window(
                        WindowSettings {
                            width: 480,
                            height: 600,
                            clear_color: window_manager::Color::rgb(0.06, 0.06, 0.08),
                            kind: WindowKind::Xdg {
                                title: "Open With".to_string(),
                            },
                            touch_config: None,
                            gesture_config: None,
                        },
                        AppChooserDialogUi::new(
                            handle.clone(),
                            app_id.clone(),
                            choices.clone(),
                            last_choice.clone(),
                            content_type.clone(),
                            uri.clone(),
                            filename.clone(),
                            pending_updates.clone(),
                        ),
                    );

                    active_windows.borrow_mut().insert(handle.clone(), id);
                    wm.flush_pending();
                }
                AppChooserRequest::UpdateChoices { handle, choices } => {
                    println!(
                        "[app-chooser-ui] UpdateChoices queued for handle={handle}: {} choices.",
                        choices.len()
                    );
                    pending_updates
                        .borrow_mut()
                        .insert(handle.clone(), choices.clone());
                }
                AppChooserRequest::Close { handle } => {
                    println!("[app-chooser-ui] Portal requested close for handle={handle}.");
                    pending_updates.borrow_mut().remove(handle);
                    let id = active_windows.borrow_mut().remove(handle);
                    if let Some(id) = id {
                        wm.destroy(id);
                    }
                }
            }
        })
        .on(move |wm: &mut WindowManager, resp: &AppChooserResponse| {
            pending_updates.borrow_mut().remove(&resp.handle);
            let id = active_windows.borrow_mut().remove(&resp.handle);
            if let Some(id) = id {
                println!(
                    "[app-chooser-ui] Dialog completed ({:?}). Closing window.",
                    resp.outcome
                );
                wm.destroy(id);
            }
        })
}

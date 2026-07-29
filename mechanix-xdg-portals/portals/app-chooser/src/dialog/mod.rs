mod widgets;
mod dialog;


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

    app::Module::<WindowManager, _, _>::new()
        .mount(ui::register_events!(AppChooserResponse))
        .on({
            let active_windows = active_windows.clone();
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
                        ),
                    );

                    active_windows.borrow_mut().insert(handle.clone(), id);
                    wm.flush_pending();
                }
                AppChooserRequest::UpdateChoices { handle, choices } => {
                    // TODO: Forward to the live dialog widget for re-rendering.
                    // Currently the update is logged; full live-update requires
                    // a shared-state bridge between the UI module and the widget.
                    println!(
                        "[app-chooser-ui] UpdateChoices for handle={handle}: {} choices.",
                        choices.len()
                    );
                }
                AppChooserRequest::Close { handle } => {
                    println!("[app-chooser-ui] Portal requested close for handle={handle}.");
                    let id = active_windows.borrow_mut().remove(handle);
                    if let Some(id) = id {
                        wm.destroy(id);
                    }
                }
            }
        })
        .on(move |wm: &mut WindowManager, resp: &AppChooserResponse| {
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

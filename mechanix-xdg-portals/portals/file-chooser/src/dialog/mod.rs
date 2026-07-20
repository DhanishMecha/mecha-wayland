mod dialog;
mod types;
pub(crate) mod widgets;

use crate::backend::{FileChooserRequest, FileChooserResponse, RequestHandle};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use window_manager::{WindowId, WindowKind, WindowManager, WindowSettings};

pub use dialog::FileChooserUi;
pub use types::ChooserOptions;

// Thread-local slots for passing results back to the application main loop.
thread_local! {
    pub static PENDING_DIALOG: Cell<Option<FileChooserResponse>> = const { Cell::new(None) };
    pub static ACTIVE_WINDOWS: RefCell<HashMap<RequestHandle, WindowId>> = RefCell::new(HashMap::new());
}

// Coordinator module to dispatch UI commands and monitor dialog actions.
pub fn filechooser_ui_module<S>() -> impl app::RegisteredModule<WindowManager, S>
where
    S: app::Lens<WindowManager> + 'static,
{
    app::Module::<WindowManager, _, _>::new()
        .on(|wm: &mut WindowManager, cmd: &FileChooserRequest| {
            let spawn_args = match cmd {
                FileChooserRequest::OpenFile {
                    handle, options, ..
                } => Some((handle.clone(), ChooserOptions::OpenFile(options.clone()))),
                FileChooserRequest::SaveFile {
                    handle, options, ..
                } => Some((handle.clone(), ChooserOptions::SaveFile(options.clone()))),
                FileChooserRequest::SaveFiles {
                    handle, options, ..
                } => Some((handle.clone(), ChooserOptions::SaveFiles(options.clone()))),
                FileChooserRequest::Close { handle } => {
                    println!("[ui] Portal requested close. Closing file chooser window.");
                    let id = ACTIVE_WINDOWS.with(|wins| wins.borrow_mut().remove(handle));
                    if let Some(id) = id {
                        wm.destroy(id);
                    }
                    None
                }
            };

            if let Some((handle, chooser_options)) = spawn_args {
                println!(
                    "[ui] Spawning file chooser window (for={:?}).",
                    chooser_options
                );

                let id = wm.spawn_window(
                    WindowSettings {
                        width: 540,
                        height: 620,
                        clear_color: window_manager::Color::rgb(0.08, 0.08, 0.1),
                        kind: WindowKind::Xdg {
                            title: "File Picker".to_string(),
                        },
                        touch_config: None,
                        gesture_config: None,
                    },
                    FileChooserUi::new(handle.clone(), chooser_options),
                );
                ACTIVE_WINDOWS.with(|wins| {
                    wins.borrow_mut().insert(handle, id);
                });
                wm.flush_pending();
            }
        })
        .on(
            |wm: &mut WindowManager, _: &app::Poll| -> Option<FileChooserResponse> {
                let done = PENDING_DIALOG.take();
                if let Some(ref done) = done {
                    let id = ACTIVE_WINDOWS.with(|wins| wins.borrow_mut().remove(&done.handle));
                    if let Some(id) = id {
                        println!("[ui] Dialog completed. Closing window and dispatching event.");
                        wm.destroy(id);
                    }
                }
                done
            },
        )
}

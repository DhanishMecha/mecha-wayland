mod dialog;

pub use dialog::InputCaptureDialogUi;

use crate::backend::types::{InputCaptureRequest, InputCaptureResponse, RequestHandle};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use window_manager::{WindowId, WindowKind, WindowManager, WindowSettings};

pub fn input_capture_ui_module<S>() -> impl app::RegisteredModule<WindowManager, S>
where
    S: app::Lens<WindowManager> + 'static,
{
    let active_windows = Rc::new(RefCell::new(HashMap::<RequestHandle, WindowId>::new()));

    app::Module::<WindowManager, _, _>::new()
        .mount(ui::register_events!(InputCaptureResponse))
        .on({
            let active_windows = active_windows.clone();
            move |wm: &mut WindowManager, cmd: &InputCaptureRequest| match cmd {
                InputCaptureRequest::Start {
                    handle,
                    session_handle: _,
                    app_id,
                    capabilities,
                } => {
                    println!(
                        "[input-capture-ui] Spawning Start dialog for app={app_id} handle={handle} caps={capabilities}."
                    );

                    let id = wm.spawn_window(
                        WindowSettings {
                            width: 420,
                            height: 400,
                            clear_color: window_manager::Color::rgb(0.08, 0.08, 0.1),
                            kind: WindowKind::Xdg {
                                title: "Input Capture Request".to_string(),
                            },
                            touch_config: None,
                            gesture_config: None,
                        },
                        InputCaptureDialogUi::new(
                            handle.clone(),
                            app_id.clone(),
                            "Input Capture".to_string(),
                            "Allow this application to capture your keyboard and pointer input?".to_string(),
                            "⌨️",
                            "Cancel",
                            "Allow",
                            *capabilities,
                        ),
                    );

                    active_windows.borrow_mut().insert(handle.clone(), id);
                    wm.flush_pending();
                }

                InputCaptureRequest::Close { handle } => {
                    println!("[input-capture-ui] Portal requested close for handle={handle}.");
                    let id = active_windows.borrow_mut().remove(handle);
                    if let Some(id) = id {
                        wm.destroy(id);
                    }
                }
            }
        })
        .on(move |wm: &mut WindowManager, resp: &InputCaptureResponse| {
            let id = active_windows.borrow_mut().remove(&resp.handle);
            if let Some(id) = id {
                println!(
                    "[input-capture-ui] Dialog completed. Closing window."
                );
                wm.destroy(id);
            }
        })
}

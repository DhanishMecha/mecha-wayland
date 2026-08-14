mod dialog;
pub mod widgets;

pub use dialog::PrintDialogUi;

use crate::backend::types::{PrintRequest, PrintResponse, RequestHandle};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use window_manager::{WindowId, WindowKind, WindowManager, WindowSettings};

pub fn print_ui_module<S>() -> impl app::RegisteredModule<WindowManager, S>
where
    S: app::Lens<WindowManager> + 'static,
{
    let active_windows = Rc::new(RefCell::new(HashMap::<RequestHandle, WindowId>::new()));

    app::Module::<WindowManager, _, _>::new()
        .mount(ui::register_events!(PrintResponse))
        .on({
            let active_windows = active_windows.clone();
            move |wm: &mut WindowManager, cmd: &PrintRequest| match cmd {
                PrintRequest::PreparePrint {
                    handle,
                    app_id,
                    title,
                    settings,
                    page_setup,
                    options,
                } => {
                    println!(
                        "[print-ui] Spawning Print dialog for app={app_id} \
                         handle={handle} title={title}."
                    );

                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        let ui = PrintDialogUi::new(
                            handle.clone(),
                            app_id.clone(),
                            title.clone(),
                            settings.clone(),
                            page_setup.clone(),
                            options.clone(),
                        );
                        wm.spawn_window(
                            WindowSettings {
                                width: 540,
                                height: 620,
                                clear_color: window_manager::Color::rgb(0.08, 0.08, 0.1),
                                kind: WindowKind::Xdg {
                                    title: "Print".to_string(),
                                },
                                touch_config: None,
                                gesture_config: None,
                            },
                            ui,
                        )
                    }));

                    match result {
                        Ok(id) => {
                            active_windows.borrow_mut().insert(handle.clone(), id);
                            wm.flush_pending();
                        }
                        Err(err) => {
                            eprintln!("[print-ui] FATAL: Panic while spawning Print dialog window: {:?}", err);
                        }
                    }
                }

                PrintRequest::Close { handle } => {
                    println!("[print-ui] Portal requested close for handle={handle}.");
                    let id = active_windows.borrow_mut().remove(handle);
                    if let Some(id) = id {
                        wm.destroy(id);
                    }
                }
            }
        })
        .on(move |wm: &mut WindowManager, resp: &PrintResponse| {
            let id = active_windows.borrow_mut().remove(&resp.handle);
            if let Some(id) = id {
                println!(
                    "[print-ui] Dialog completed. Closing window."
                );
                wm.destroy(id);
            }
        })
}

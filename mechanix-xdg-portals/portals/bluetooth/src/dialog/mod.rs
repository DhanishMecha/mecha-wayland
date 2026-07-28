mod dialog;
mod types;

use crate::backend::{BluetoothRequest, BluetoothResponse};
use std::cell::Cell;
use std::rc::Rc;
use window_manager::{WindowId, WindowKind, WindowManager, WindowSettings};

pub use dialog::BluetoothDialogUi;
pub use types::DialogArgs;

// Coordinator module

/// Mount this on the `WindowManager` state slice (exactly like
/// `filechooser_ui_module`).
pub fn bluetooth_ui_module<S>() -> impl app::RegisteredModule<WindowManager, S>
where
    S: app::Lens<WindowManager> + 'static,
{
    let active: Rc<Cell<Option<WindowId>>> = Rc::new(Cell::new(None));

    app::Module::<WindowManager, _, _>::new()
        .mount(ui::register_events!(BluetoothResponse))
        .on({
            let active = active.clone();
            move |wm: &mut WindowManager, req: &BluetoothRequest| match req {
                BluetoothRequest::Cancel => {
                    if let Some(id) = active.take() {
                        wm.destroy(id);
                    }
                }
                _ => {
                    let Some(args) = DialogArgs::from_request(req) else {
                        return;
                    };
                    // Close any existing dialog before opening a new one.
                    if let Some(old) = active.take() {
                        wm.destroy(old);
                    }
                    let title = format!("Bluetooth — {}", args.device);
                    let id = wm.spawn_window(
                        WindowSettings {
                            width: 540,
                            height: 620,
                            clear_color: window_manager::Color::TRANSPARENT,
                            kind: WindowKind::Xdg { title },
                            touch_config: None,
                            gesture_config: None,
                        },
                        BluetoothDialogUi::new(&args.device, &args.kind),
                    );
                    active.set(Some(id));
                    wm.flush_pending();
                }
            }
        })
        .on(move |wm: &mut WindowManager, _: &BluetoothResponse| {
            if let Some(id) = active.take() {
                wm.destroy(id);
            }
        })
}

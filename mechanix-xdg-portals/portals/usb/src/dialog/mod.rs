mod dialog;
mod helpers;
pub mod widgets;

pub use dialog::UsbDialogUi;

use crate::backend::types::{RequestHandle, UsbRequest, UsbResponse};
use helpers::parse_device;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use window_manager::{WindowId, WindowKind, WindowManager, WindowSettings};

pub fn usb_ui_module<S>() -> impl app::RegisteredModule<WindowManager, S>
where
    S: app::Lens<WindowManager> + 'static,
{
    let active_windows = Rc::new(RefCell::new(HashMap::<RequestHandle, WindowId>::new()));

    app::Module::<WindowManager, _, _>::new()
        .mount(ui::register_events!(UsbResponse))
        .on({
            let active_windows = active_windows.clone();
            move |wm: &mut WindowManager, cmd: &UsbRequest| match cmd {
                UsbRequest::AcquireDevices(info) => {
                    println!(
                        "[usb-ui] Spawning dialog for app={} (handle={})",
                        info.app_id, info.handle
                    );

                    let devices_ui = info
                        .devices
                        .iter()
                        .map(|(dev_id, info_dict, access_opts)| {
                            parse_device(dev_id, info_dict, access_opts)
                        })
                        .collect();

                    let id = wm.spawn_window(
                        WindowSettings {
                            width: 500,
                            height: 560,
                            clear_color: window_manager::Color::rgb(0.08, 0.08, 0.1),
                            kind: WindowKind::Xdg {
                                title: "USB Device Access".to_string(),
                            },
                            touch_config: None,
                            gesture_config: None,
                        },
                        UsbDialogUi::new(info.handle.clone(), info.app_id.clone(), devices_ui),
                    );

                    active_windows.borrow_mut().insert(info.handle.clone(), id);
                    wm.flush_pending();
                }

                UsbRequest::Close { handle } => {
                    if let Some(id) = active_windows.borrow_mut().remove(handle) {
                        wm.destroy(id);
                    }
                }
            }
        })
        .on(move |wm: &mut WindowManager, resp: &UsbResponse| {
            if let Some(id) = active_windows.borrow_mut().remove(&resp.handle) {
                wm.destroy(id);
            }
        })
}

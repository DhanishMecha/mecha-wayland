mod dialog;

pub use dialog::NotificationBannerUi;

use crate::backend::types::{NotificationRequest, NotificationResponse};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use window_manager::{
    WindowId, WindowKind, WindowManager, WindowSettings, ZwlrLayerShellV1Layer,
    ZwlrLayerSurfaceV1Anchor, ZwlrLayerSurfaceV1KeyboardInteractivity,
};

pub fn notification_ui_module<S>() -> impl app::RegisteredModule<WindowManager, S>
where
    S: app::Lens<WindowManager> + 'static,
{
    // Keeps track of active notification windows: (app_id, id) -> (WindowId, expiration_time)
    let active_windows = Rc::new(RefCell::new(HashMap::<
        (String, String),
        (WindowId, std::time::Instant),
    >::new()));

    app::Module::<WindowManager, _, _>::new()
        .mount(ui::register_events!(NotificationResponse))
        .on({
            let active_windows = active_windows.clone();
            move |wm: &mut WindowManager, cmd: &NotificationRequest| match cmd {
                NotificationRequest::Add { app_id, id, info } => {
                    println!(
                        "[notification-ui] Spawning notification banner for app={app_id} id={id}"
                    );
                    let key = (app_id.clone(), id.clone());
                    if let Some((win_id, _)) = active_windows.borrow_mut().remove(&key) {
                        wm.destroy(win_id);
                    }

                    let kind = if wm.has_layer_shell() {
                        WindowKind::LayerShell {
                            layer: ZwlrLayerShellV1Layer::Overlay,
                            anchor: ZwlrLayerSurfaceV1Anchor::Top,
                            exclusive_zone: 0,
                            namespace: "notification".to_string(),
                            keyboard_interactivity: ZwlrLayerSurfaceV1KeyboardInteractivity::None,
                        }
                    } else {
                        WindowKind::Xdg {
                            title: format!("Notification - {}", app_id),
                        }
                    };

                    let height = if info.buttons.is_empty() { 110 } else { 160 };
                    let win_id = wm.spawn_window(
                        WindowSettings {
                            width: 380,
                            height,
                            clear_color: window_manager::Color::BLACK,
                            kind,
                            touch_config: None,
                            gesture_config: None,
                        },
                        NotificationBannerUi::new(app_id.clone(), id.clone(), info.clone()),
                    );

                    // Auto-dismiss after 5 seconds
                    let expire_time =
                        std::time::Instant::now() + std::time::Duration::from_secs(15);
                    active_windows
                        .borrow_mut()
                        .insert(key, (win_id, expire_time));
                    wm.flush_pending();
                }
                NotificationRequest::Remove { app_id, id } => {
                    let key = (app_id.clone(), id.clone());
                    if let Some((win_id, _)) = active_windows.borrow_mut().remove(&key) {
                        println!(
                            "[notification-ui] Closing notification banner for app={app_id} id={id}"
                        );
                        wm.destroy(win_id);
                    }
                }
            }
        })
        .on({
            let active_windows = active_windows.clone();
            move |wm: &mut WindowManager, resp: &NotificationResponse| {
                let key = (resp.app_id.clone(), resp.id.clone());
                if let Some((win_id, _)) = active_windows.borrow_mut().remove(&key) {
                    println!("[notification-ui] Action handled. Closing banner window.");
                    wm.destroy(win_id);
                }
            }
        })
        .on({
            let active_windows = active_windows.clone();
            move |wm: &mut WindowManager, _: &app::Poll| {
                let mut to_remove = Vec::new();
                let now = std::time::Instant::now();
                {
                    let borrowed = active_windows.borrow();
                    for (key, (_win_id, expire_time)) in borrowed.iter() {
                        if now >= *expire_time {
                            to_remove.push(key.clone());
                        }
                    }
                }
                for key in to_remove {
                    if let Some((win_id, _)) = active_windows.borrow_mut().remove(&key) {
                        println!("[notification-ui] Notification timeout. Closing banner window.");
                        wm.destroy(win_id);
                    }
                }
            }
        })
}

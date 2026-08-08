mod dialog;

pub use dialog::WallpaperDialogUi;

use crate::backend::types::{RequestHandle, WallpaperRequest, WallpaperResponse};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use window_manager::{WindowId, WindowKind, WindowManager, WindowSettings};

pub fn wallpaper_ui_module<S>() -> impl app::RegisteredModule<WindowManager, S>
where
    S: app::Lens<WindowManager> + 'static,
{
    let active_windows = Rc::new(RefCell::new(HashMap::<RequestHandle, WindowId>::new()));

    app::Module::<WindowManager, _, _>::new()
        .mount(ui::register_events!(WallpaperResponse))
        .on({
            let active_windows = active_windows.clone();
            move |wm: &mut WindowManager, cmd: &WallpaperRequest| match cmd {
                WallpaperRequest::SetWallpaper {
                    handle,
                    app_id,
                    uri,
                    show_preview,
                    set_on,
                } => {
                    println!(
                        "[wallpaper-ui] Spawning SetWallpaper dialog for app={app_id} \
                         handle={handle} uri={uri} show_preview={show_preview} set_on={set_on}."
                    );

                    let body = match set_on.as_str() {
                        "lockscreen" => "Lock Screen",
                        "both" => "Desktop & Lock Screen",
                        _ => "Desktop",
                    };

                    let id = wm.spawn_window(
                        WindowSettings {
                            width: 440,
                            height: 350,
                            clear_color: window_manager::Color::rgb(0.08, 0.08, 0.1),
                            kind: WindowKind::Xdg {
                                title: "Set Wallpaper".to_string(),
                            },
                            touch_config: None,
                            gesture_config: None,
                        },
                        WallpaperDialogUi::new(
                            handle.clone(),
                            app_id.clone(),
                            "Set Wallpaper".to_string(),
                            body.to_string(),
                            uri.clone(),
                            "🖼️",
                            "Cancel",
                            "Set Wallpaper",
                        ),
                    );

                    active_windows.borrow_mut().insert(handle.clone(), id);
                    wm.flush_pending();
                }

                WallpaperRequest::Close { handle } => {
                    println!("[wallpaper-ui] Portal requested close for handle={handle}.");
                    let id = active_windows.borrow_mut().remove(handle);
                    if let Some(id) = id {
                        wm.destroy(id);
                    }
                }
            }
        })
        .on(move |wm: &mut WindowManager, resp: &WallpaperResponse| {
            let id = active_windows.borrow_mut().remove(&resp.handle);
            if let Some(id) = id {
                println!(
                    "[wallpaper-ui] Dialog completed ({:?}). Closing window.",
                    resp.outcome
                );
                wm.destroy(id);
            }
        })
}

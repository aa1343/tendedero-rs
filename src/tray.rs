use std::sync::atomic::Ordering;
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter,
};
use crate::mouse_tracker::IS_REVEALED;

pub fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let toggle_item = MenuItem::with_id(app, "toggle", "展开 / 收起晾衣绳 (Ctrl+Alt+T)", true, None::<&str>)?;
    let capture_item = MenuItem::with_id(app, "capture", "立即截屏挂绳 (Ctrl+Alt+S)", true, None::<&str>)?;
    let todo_item = MenuItem::with_id(app, "todo", "新建待办便签 (Ctrl+Alt+N)", true, None::<&str>)?;
    let search_item = MenuItem::with_id(app, "search", "绳上搜索 (Ctrl+Alt+F)", true, None::<&str>)?;
    let separator1 = PredefinedMenuItem::separator(app)?;

    let is_auto = crate::autostart::is_autostart_enabled();
    let autostart_item = CheckMenuItem::with_id(
        app,
        "autostart",
        "开机自启动",
        true,
        is_auto,
        None::<&str>,
    )?;

    let data_folder = MenuItem::with_id(app, "folder", "打开数据存储目录", true, None::<&str>)?;
    let separator2 = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出 Tendedero", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &toggle_item,
            &capture_item,
            &todo_item,
            &search_item,
            &separator1,
            &autostart_item,
            &data_folder,
            &separator2,
            &quit_item,
        ],
    )?;

    let icon_bytes = include_bytes!("../icons/32x32.png");
    let icon = tauri::image::Image::from_bytes(icon_bytes)?;

    let autostart_item_clone = autostart_item.clone();

    let _tray = TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .tooltip("Tendedero (Rust) - 晾衣绳")
        .show_menu_on_left_click(false)
        .on_menu_event(move |app_handle, event| {
            let id = event.id.as_ref();
            match id {
                "toggle" => {
                    let cur = IS_REVEALED.load(Ordering::SeqCst);
                    let next = !cur;
                    crate::mouse_tracker::set_revealed_state(app_handle, next);
                }
                "capture" => {
                    let _ = app_handle.emit("trigger_capture", ());
                }
                "todo" => {
                    crate::mouse_tracker::set_revealed_state(app_handle, true);
                    let _ = app_handle.emit("open_new_todo", ());
                }
                "search" => {
                    crate::mouse_tracker::set_revealed_state(app_handle, true);
                    let _ = app_handle.emit("open_search", ());
                }
                "autostart" => {
                    match crate::autostart::toggle_autostart() {
                        Ok(enabled) => {
                            let _ = autostart_item_clone.set_checked(enabled);
                            let msg = if enabled {
                                "已成功开启开机自启动"
                            } else {
                                "已成功关闭开机自启动"
                            };
                            let _ = app_handle.emit("show_toast", msg);
                        }
                        Err(e) => {
                            eprintln!("切换开机自启动失败: {}", e);
                            let _ = app_handle.emit("show_toast", format!("设置自启失败: {}", e));
                        }
                    }
                }
                "folder" => {
                    let _ = app_handle.emit("open_data_folder", ());
                }
                "quit" => {
                    app_handle.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                let cur = IS_REVEALED.load(Ordering::SeqCst);
                let next = !cur;
                crate::mouse_tracker::set_revealed_state(app, next);
            }
        })
        .build(app)?;

    Ok(())
}

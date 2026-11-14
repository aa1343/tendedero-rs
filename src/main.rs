// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod autostart;
mod capture;
mod clipboard;
mod commands;
mod hotkey;
mod models;
mod mouse_tracker;
mod storage;
mod tray;

use tauri::{Manager, PhysicalPosition, PhysicalSize};
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN};
use crate::storage::StorageManager;

fn main() {
    // 0. 全局异常捕获弹窗与日志记录，确保任何异常绝不静默闪退
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("Tendedero 运行时遇到异常：\n{}", info);
        if let Some(temp) = std::env::var_os("TEMP") {
            let log_file = std::path::Path::new(&temp).join("tendedero_crash.log");
            let _ = std::fs::write(&log_file, &msg);
        }
        unsafe {
            use windows::core::HSTRING;
            use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
            let _ = MessageBoxW(
                None,
                &HSTRING::from(msg),
                &HSTRING::from("Tendedero 错误提示"),
                MB_OK | MB_ICONERROR,
            );
        }
    }));

    // 1. 确保安全清理启动参数：绝对不可包含 --disable-gpu（会破坏 Windows DWM 透明 Alpha 混合通道），
    // 并自动补齐管理员权限必需的 --no-sandbox
    let current_args = std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").unwrap_or_default();
    let cleaned_args = current_args
        .replace("--disable-gpu", "")
        .replace("  ", " ");
    let final_args = if !cleaned_args.contains("--no-sandbox") {
        format!("{} --no-sandbox", cleaned_args.trim())
    } else {
        cleaned_args
    };
    std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", final_args.trim());

    // 2. 隔离 WebView2 用户数据目录至 TEMP 临时目录，并由 Rust 自动补齐 Everyone 读写授权
    // 确保用户直接双击 .exe 也能 100% 成功启动，绝不闪退！
    if let Some(temp) = std::env::var_os("TEMP") {
        let user_data = std::path::Path::new(&temp).join("tendedero_data");
        let _ = std::fs::create_dir_all(&user_data);
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("icacls.exe")
            .arg(&user_data)
            .arg("/grant")
            .arg("Everyone:(OI)(CI)F")
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output();
        std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", &user_data);
    }

    let storage = StorageManager::new();

    tauri::Builder::default()
        .manage(storage.clone())
        .invoke_handler(tauri::generate_handler![
            commands::get_cards,
            commands::get_settings,
            commands::save_settings,
            commands::remove_card,
            commands::undo_remove,
            commands::toggle_pin,
            commands::capture_screen_now,
            commands::copy_card_content,
            commands::open_card,
            commands::open_file_location,
            commands::open_data_folder,
            commands::set_revealed,
            commands::set_locked,
            commands::set_ignore_cursor,
            commands::create_todo,
            commands::update_todo_item,
            commands::add_todo_item,
            commands::delete_todo_item,
            commands::update_todo_title,
            commands::update_todo_color,
            commands::get_autostart_status,
            commands::toggle_autostart,
            commands::focus_window,
        ])
        .setup(move |app| {
            let app_handle = app.handle().clone();

            // 动态适配屏幕宽度并初始化为展开状态
            if let Some(window) = app.get_webview_window("main") {
                let screen_w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
                let width = if screen_w > 0 { screen_w as u32 } else { 1920 };
                let _ = window.set_position(tauri::Position::Physical(PhysicalPosition { x: 0, y: 0 }));
                let _ = window.set_size(tauri::Size::Physical(PhysicalSize {
                    width,
                    height: 420,
                }));
                // 启动初始直接展开，用户一眼即可看到完整的晾衣绳与卡片，并可直接操作！
                let _ = window.set_ignore_cursor_events(false);
            }

            // Set up system tray
            if let Err(e) = tray::setup_tray(&app_handle) {
                eprintln!("Failed to setup tray: {}", e);
            }

            // Start background tasks
            clipboard::start_clipboard_monitor(app_handle.clone(), storage.clone());
            mouse_tracker::start_mouse_tracker(app_handle.clone(), storage.clone());
            hotkey::start_hotkey_listener(app_handle.clone(), storage.clone());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

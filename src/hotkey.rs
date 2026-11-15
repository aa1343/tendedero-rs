use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, TranslateMessage, MSG, WM_HOTKEY,
};
use crate::capture::capture_primary_screen;
use crate::models::{Card, CardType};
use crate::mouse_tracker::IS_REVEALED;
use crate::storage::StorageManager;
use chrono::Local;

const HOTKEY_ID_TOGGLE: i32 = 1001;
const HOTKEY_ID_CAPTURE: i32 = 1002;
const HOTKEY_ID_TODO: i32 = 1003;
const HOTKEY_ID_SEARCH: i32 = 1004;

pub fn start_hotkey_listener(app_handle: AppHandle, storage: Arc<StorageManager>) {
    std::thread::spawn(move || unsafe {
        let hwnd = HWND(std::ptr::null_mut());
        let mods: HOT_KEY_MODIFIERS = MOD_CONTROL | MOD_ALT;

        // 'T' = 0x54, 'S' = 0x53, 'N' = 0x4E, 'F' = 0x46
        let _ = RegisterHotKey(hwnd, HOTKEY_ID_TOGGLE, mods, 0x54);
        let _ = RegisterHotKey(hwnd, HOTKEY_ID_CAPTURE, mods, 0x53);
        let _ = RegisterHotKey(hwnd, HOTKEY_ID_TODO, mods, 0x4E);
        let _ = RegisterHotKey(hwnd, HOTKEY_ID_SEARCH, mods, 0x46);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, hwnd, 0, 0).as_bool() {
            if msg.message == WM_HOTKEY {
                let id = msg.wParam.0 as i32;
                match id {
                    HOTKEY_ID_TOGGLE => {
                        let cur = IS_REVEALED.load(Ordering::SeqCst);
                        let next = !cur;
                        crate::mouse_tracker::set_revealed_state(&app_handle, next);
                    }
                    HOTKEY_ID_CAPTURE => {
                        let shots_dir = storage.get_screenshots_dir();
                        if let Ok(res) = capture_primary_screen(&shots_dir) {
                            let now = Local::now().timestamp_millis();
                            let card = Card {
                                id: format!("shot-{}", uuid::Uuid::new_v4()),
                                card_type: CardType::Screenshot,
                                title: format!("屏幕截图 {}", Local::now().format("%H:%M:%S")),
                                content: res.file_path.to_string_lossy().to_string(),
                                thumbnail_data: Some(res.thumbnail_data),
                                meta: Some(format!("{}x{} · {} KB", res.width, res.height, res.size_bytes / 1024)),
                                pinned: false,
                                created_at: now,
                                file_path: Some(res.file_path.to_string_lossy().to_string()),
                                todo_data: None,
                            };
                            storage.add_card(card.clone());
                            crate::mouse_tracker::trigger_preview(&app_handle, 3500);
                            let _ = app_handle.emit("card_added", &card);
                        }
                    }
                    HOTKEY_ID_TODO => {
                        crate::mouse_tracker::set_revealed_state(&app_handle, true);
                        let _ = app_handle.emit("open_new_todo", ());
                    }
                    HOTKEY_ID_SEARCH => {
                        crate::mouse_tracker::set_revealed_state(&app_handle, true);
                        let _ = app_handle.emit("open_search", ());
                    }
                    _ => {}
                }
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });
}

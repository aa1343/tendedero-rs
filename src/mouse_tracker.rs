use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
use crate::storage::StorageManager;

pub static IS_REVEALED: AtomicBool = AtomicBool::new(true);
pub static IS_LOCKED: AtomicBool = AtomicBool::new(false); // When modal/menu is open

/// 统一管理晾衣绳的展开与收起状态，并精确同步鼠标穿透
pub fn set_revealed_state(app_handle: &AppHandle, revealed: bool) {
    IS_REVEALED.store(revealed, Ordering::SeqCst);
    if let Some(window) = app_handle.get_webview_window("main") {
        // 展开时允许交互（不穿透），收起时完全穿透鼠标，彻底杜绝阻挡正常使用
        let _ = window.set_ignore_cursor_events(!revealed);
    }
    let _ = app_handle.emit("set_revealed", revealed);
}

pub fn start_mouse_tracker(app_handle: AppHandle, storage: Arc<StorageManager>) {
    std::thread::spawn(move || {
        let mut dwell_start: Option<Instant> = None;
        let mut away_start: Option<Instant> = None;

        loop {
            std::thread::sleep(Duration::from_millis(30));

            let opens_from_top = storage.settings.lock().unwrap().opens_from_top_edge;
            let mut pt = POINT { x: 0, y: 0 };
            let cursor_ok = unsafe { GetCursorPos(&mut pt) }.is_ok();

            if !cursor_ok {
                continue;
            }

            let is_revealed = IS_REVEALED.load(Ordering::SeqCst);
            let is_locked = IS_LOCKED.load(Ordering::SeqCst);

            // 1. 指针在屏幕顶边 (y <= 8) 停留 200ms，平滑滑下晾衣绳
            if opens_from_top && pt.y <= 8 && !is_revealed {
                if let Some(start) = dwell_start {
                    if start.elapsed() >= Duration::from_millis(200) {
                        dwell_start = None;
                        set_revealed_state(&app_handle, true);
                    }
                } else {
                    dwell_start = Some(Instant::now());
                }
            } else {
                dwell_start = None;
            }

            // 2. 移开鼠标到下方 (y > 340) 持续 450ms，自动收回并恢复穿透
            if is_revealed && !is_locked {
                if pt.y > 340 {
                    if let Some(start) = away_start {
                        if start.elapsed() >= Duration::from_millis(450) {
                            away_start = None;
                            set_revealed_state(&app_handle, false);
                        }
                    } else {
                        away_start = Some(Instant::now());
                    }
                } else {
                    away_start = None;
                }
            } else {
                away_start = None;
            }
        }
    });
}

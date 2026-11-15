use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
use crate::storage::StorageManager;

pub static IS_REVEALED: AtomicBool = AtomicBool::new(true);
pub static IS_LOCKED: AtomicBool = AtomicBool::new(false); // When modal/menu is open
pub static PREVIEW_UNTIL: AtomicI64 = AtomicI64::new(0); // 截图或新增内容时的临时预览截止时间戳 (ms)

/// 触发新内容挂绳预览（默认 3500ms），若用户未将光标移至绳上操作，将在此时间后自动优雅缩回
pub fn trigger_preview(app_handle: &AppHandle, duration_ms: i64) {
    let now = chrono::Local::now().timestamp_millis();
    PREVIEW_UNTIL.store(now + duration_ms, Ordering::SeqCst);
    set_revealed_state(app_handle, true);
}

/// 统一管理晾衣绳的展开与收起状态，并精确同步鼠标穿透
pub fn set_revealed_state(app_handle: &AppHandle, revealed: bool) {
    if !revealed {
        PREVIEW_UNTIL.store(0, Ordering::SeqCst);
    }
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
            std::thread::sleep(Duration::from_millis(16));

            let opens_from_top = storage.settings.lock().unwrap().opens_from_top_edge;
            let mut pt = POINT { x: 0, y: 0 };
            let cursor_ok = unsafe { GetCursorPos(&mut pt) }.is_ok();

            if !cursor_ok {
                continue;
            }

            let is_revealed = IS_REVEALED.load(Ordering::SeqCst);
            let is_locked = IS_LOCKED.load(Ordering::SeqCst);
            let preview_until = PREVIEW_UNTIL.load(Ordering::SeqCst);
            let now_ms = chrono::Local::now().timestamp_millis();

            // 1. 指针在屏幕顶边 (y <= 8) 停留 180ms，平滑滑下晾衣绳
            if opens_from_top && pt.y <= 8 && !is_revealed {
                if let Some(start) = dwell_start {
                    if start.elapsed() >= Duration::from_millis(180) {
                        dwell_start = None;
                        PREVIEW_UNTIL.store(0, Ordering::SeqCst);
                        set_revealed_state(&app_handle, true);
                    }
                } else {
                    dwell_start = Some(Instant::now());
                }
            } else {
                dwell_start = None;
            }

            // 用户开始使用晾衣绳：如果光标移入了绳上卡片区域 (y <= 260)，立即取消自动缩回倒计时，
            // 只要鼠标停留在上方，就一直保持展开供用户查看、点击、拖拽与操作
            if is_revealed && pt.y <= 260 && preview_until > 0 {
                PREVIEW_UNTIL.store(0, Ordering::SeqCst);
            }

            // 2. 指针移开到下方 (y > 260) 或截图预览期超时自动缩回：
            if is_revealed && !is_locked {
                if preview_until > 0 {
                    // 处于截图/复制等新增内容的临时预览期：
                    if now_ms >= preview_until {
                        // 用户未触碰绳子，预览期满自动优雅收回并恢复穿透
                        PREVIEW_UNTIL.store(0, Ordering::SeqCst);
                        away_start = None;
                        set_revealed_state(&app_handle, false);
                    }
                } else if pt.y > 260 {
                    // 常规状态：光标移开到绳子下方，40ms 极速缩回
                    if let Some(start) = away_start {
                        if start.elapsed() >= Duration::from_millis(40) {
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

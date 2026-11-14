use std::path::Path;
use std::process::Command;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use chrono::Local;
use tauri::{AppHandle, Emitter, State, WebviewWindow};
use crate::capture::capture_primary_screen;
use crate::clipboard::{copy_image_to_clipboard, copy_text_to_clipboard};
use crate::models::{AppSettings, Card, CardType, TodoCard, TodoItem};
use crate::mouse_tracker::{IS_LOCKED, IS_REVEALED};
use crate::storage::StorageManager;

#[tauri::command]
pub fn get_cards(storage: State<Arc<StorageManager>>) -> Vec<Card> {
    storage.cards.lock().unwrap().clone()
}

#[tauri::command]
pub fn get_settings(storage: State<Arc<StorageManager>>) -> AppSettings {
    storage.settings.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_settings(storage: State<Arc<StorageManager>>, new_settings: AppSettings) {
    {
        let mut s = storage.settings.lock().unwrap();
        *s = new_settings;
    }
    storage.save_settings();
}

#[tauri::command]
pub fn remove_card(storage: State<Arc<StorageManager>>, id: String) -> Option<Card> {
    storage.remove_card(&id)
}

#[tauri::command]
pub fn undo_remove(storage: State<Arc<StorageManager>>) -> Option<Card> {
    storage.undo_remove()
}

#[tauri::command]
pub fn toggle_pin(storage: State<Arc<StorageManager>>, id: String) -> Option<bool> {
    storage.toggle_pin(&id)
}

#[tauri::command]
pub fn capture_screen_now(app: AppHandle, storage: State<Arc<StorageManager>>) -> Result<Card, String> {
    let shots_dir = storage.get_screenshots_dir();
    let res = capture_primary_screen(&shots_dir)?;
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
    let _ = app.emit("card_added", &card);
    Ok(card)
}

#[tauri::command]
pub fn copy_card_content(storage: State<Arc<StorageManager>>, id: String) -> Result<String, String> {
    let cards = storage.cards.lock().unwrap();
    let card = cards.iter().find(|c| c.id == id).ok_or("Card not found")?;

    match card.card_type {
        CardType::Screenshot | CardType::Image => {
            if let Some(ref path_str) = card.file_path {
                let path = Path::new(path_str);
                copy_image_to_clipboard(path)?;
                Ok("已复制图片到剪贴板".to_string())
            } else {
                Err("Image file not found".to_string())
            }
        }
        CardType::Note | CardType::Link => {
            copy_text_to_clipboard(&card.content)?;
            Ok("已复制内容到剪贴板".to_string())
        }
        CardType::Todo => {
            if let Some(ref todo) = card.todo_data {
                let mut lines = Vec::new();
                lines.push(format!("# {}", todo.title));
                for item in &todo.items {
                    lines.push(format!("- [{}] {}", if item.done { "x" } else { " " }, item.text));
                }
                let full = lines.join("\n");
                copy_text_to_clipboard(&full)?;
                Ok("已复制待办清单到剪贴板".to_string())
            } else {
                copy_text_to_clipboard(&card.title)?;
                Ok("已复制".to_string())
            }
        }
    }
}

#[tauri::command]
pub fn open_card(storage: State<Arc<StorageManager>>, id: String) -> Result<(), String> {
    let cards = storage.cards.lock().unwrap();
    let card = cards.iter().find(|c| c.id == id).ok_or("Card not found")?;

    match card.card_type {
        CardType::Link => {
            open::that(&card.content).map_err(|e| e.to_string())?;
        }
        CardType::Screenshot | CardType::Image | CardType::Note => {
            if let Some(ref path_str) = card.file_path {
                open::that(path_str).map_err(|e| e.to_string())?;
            }
        }
        CardType::Todo => {
            // Handled in UI
        }
    }
    Ok(())
}

#[tauri::command]
pub fn open_file_location(storage: State<Arc<StorageManager>>, id: String) -> Result<(), String> {
    let cards = storage.cards.lock().unwrap();
    let card = cards.iter().find(|c| c.id == id).ok_or("Card not found")?;

    if let Some(ref path_str) = card.file_path {
        let _ = Command::new("explorer.exe")
            .arg(format!("/select,{}", path_str))
            .spawn();
    } else {
        let _ = Command::new("explorer.exe")
            .arg(storage.get_base_dir())
            .spawn();
    }
    Ok(())
}

#[tauri::command]
pub fn open_data_folder(storage: State<Arc<StorageManager>>) {
    let _ = Command::new("explorer.exe")
        .arg(storage.get_base_dir())
        .spawn();
}

#[tauri::command]
pub fn set_revealed(app: AppHandle, revealed: bool) {
    crate::mouse_tracker::set_revealed_state(&app, revealed);
}

#[tauri::command]
pub fn set_locked(locked: bool) {
    IS_LOCKED.store(locked, Ordering::SeqCst);
}

#[tauri::command]
pub fn set_ignore_cursor(window: WebviewWindow, ignore: bool) {
    let _ = window.set_ignore_cursor_events(ignore);
}

// ---------------------------------------------------------------------------
// TODO便签专用功能
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn create_todo(
    app: AppHandle,
    storage: State<Arc<StorageManager>>,
    title: String,
    color: String,
    items: Vec<String>,
) -> Card {
    let now = Local::now().timestamp_millis();
    let todo_items: Vec<TodoItem> = items
        .into_iter()
        .enumerate()
        .map(|(idx, text)| TodoItem {
            id: format!("{}-{}", now, idx),
            text,
            done: false,
        })
        .collect();

    let total = todo_items.len();
    let todo_card = TodoCard {
        id: format!("todo-{}", uuid::Uuid::new_v4()),
        title: if title.trim().is_empty() { "待办便签".to_string() } else { title.clone() },
        color,
        items: todo_items,
        created_at: now,
        updated_at: now,
    };

    let card = Card {
        id: format!("todo-{}", uuid::Uuid::new_v4()),
        card_type: CardType::Todo,
        title: todo_card.title.clone(),
        content: String::new(),
        thumbnail_data: None,
        meta: Some(format!("0/{} 已完成", total)),
        pinned: true, // Default to pinned so auto-clean does not drop important tasks
        created_at: now,
        file_path: None,
        todo_data: Some(todo_card),
    };

    storage.add_card(card.clone());
    let _ = app.emit("card_added", &card);
    card
}

#[tauri::command]
pub fn update_todo_item(
    storage: State<Arc<StorageManager>>,
    card_id: String,
    item_id: String,
    done: bool,
) -> Option<Card> {
    storage.update_todo(&card_id, |todo| {
        if let Some(item) = todo.items.iter_mut().find(|i| i.id == item_id) {
            item.done = done;
        }
    })
}

#[tauri::command]
pub fn add_todo_item(
    storage: State<Arc<StorageManager>>,
    card_id: String,
    text: String,
) -> Option<Card> {
    if text.trim().is_empty() {
        return None;
    }
    let now = Local::now().timestamp_millis();
    let new_id = format!("{}-{}", now, uuid::Uuid::new_v4());
    storage.update_todo(&card_id, |todo| {
        todo.items.push(TodoItem {
            id: new_id,
            text: text.trim().to_string(),
            done: false,
        });
    })
}

#[tauri::command]
pub fn delete_todo_item(
    storage: State<Arc<StorageManager>>,
    card_id: String,
    item_id: String,
) -> Option<Card> {
    storage.update_todo(&card_id, |todo| {
        todo.items.retain(|i| i.id != item_id);
    })
}

#[tauri::command]
pub fn update_todo_title(
    storage: State<Arc<StorageManager>>,
    card_id: String,
    title: String,
) -> Option<Card> {
    if title.trim().is_empty() {
        return None;
    }
    storage.update_todo(&card_id, |todo| {
        todo.title = title.trim().to_string();
    })
}

#[tauri::command]
pub fn update_todo_color(
    storage: State<Arc<StorageManager>>,
    card_id: String,
    color: String,
) -> Option<Card> {
    storage.update_todo(&card_id, |todo| {
        todo.color = color;
    })
}

#[tauri::command]
pub fn get_autostart_status() -> bool {
    crate::autostart::is_autostart_enabled()
}

#[tauri::command]
pub fn toggle_autostart() -> Result<bool, String> {
    crate::autostart::toggle_autostart()
}

#[tauri::command]
pub fn focus_window(window: WebviewWindow) {
    let _ = window.set_focus();
    #[cfg(target_os = "windows")]
    unsafe {
        if let Ok(hwnd) = window.hwnd() {
            use windows::Win32::UI::WindowsAndMessaging::{SetActiveWindow, SetForegroundWindow};
            let _ = SetActiveWindow(hwnd);
            let _ = SetForegroundWindow(hwnd);
        }
    }
}

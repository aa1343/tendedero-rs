use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use arboard::Clipboard;
use chrono::Local;
use tauri::{AppHandle, Emitter};
use crate::models::{Card, CardType};
use crate::storage::StorageManager;

static WATCHING: AtomicBool = AtomicBool::new(true);

#[allow(dead_code)]
pub fn set_clipboard_watching(enabled: bool) {
    WATCHING.store(enabled, Ordering::SeqCst);
}

pub fn copy_text_to_clipboard(text: &str) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_text(text).map_err(|e| e.to_string())
}

pub fn copy_image_to_clipboard(path: &Path) -> Result<(), String> {
    let img = image::open(path).map_err(|e| e.to_string())?.to_rgba8();
    let (width, height) = img.dimensions();
    let img_data = arboard::ImageData {
        width: width as usize,
        height: height as usize,
        bytes: std::borrow::Cow::Owned(img.into_raw()),
    };
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_image(img_data).map_err(|e| e.to_string())
}

pub fn start_clipboard_monitor(app_handle: AppHandle, storage: Arc<StorageManager>) {
    std::thread::spawn(move || {
        let mut last_text = String::new();
        let mut last_image_hash: u64 = 0;

        loop {
            std::thread::sleep(Duration::from_millis(600));

            if !WATCHING.load(Ordering::SeqCst) {
                continue;
            }

            let settings = storage.settings.lock().unwrap().clone();
            let mut clipboard = match Clipboard::new() {
                Ok(c) => c,
                Err(_) => continue,
            };

            // 1. Check for text/links
            if settings.watch_clipboard_text || settings.watch_clipboard_link {
                if let Ok(text) = clipboard.get_text() {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() && trimmed != last_text {
                        last_text = trimmed.to_string();

                        let is_link = trimmed.starts_with("http://") || trimmed.starts_with("https://");
                        if is_link && settings.watch_clipboard_link {
                            let domain = trimmed
                                .trim_start_matches("http://")
                                .trim_start_matches("https://")
                                .split('/')
                                .next()
                                .unwrap_or("网页链接");

                            let now = Local::now().timestamp_millis();
                            let card = Card {
                                id: format!("link-{}", uuid::Uuid::new_v4()),
                                card_type: CardType::Link,
                                title: domain.to_string(),
                                content: trimmed.to_string(),
                                thumbnail_data: None,
                                meta: Some("网页链接".to_string()),
                                pinned: false,
                                created_at: now,
                                file_path: None,
                                todo_data: None,
                            };

                            storage.add_card(card.clone());
                            crate::mouse_tracker::trigger_preview(&app_handle, 3500);
                            let _ = app_handle.emit("card_added", &card);
                        } else if !is_link && settings.watch_clipboard_text {
                            let now = Local::now().timestamp_millis();
                            let first_line = trimmed.lines().next().unwrap_or("复制的文本");
                            let title = if first_line.chars().count() > 30 {
                                format!("{}...", first_line.chars().take(30).collect::<String>())
                            } else {
                                first_line.to_string()
                            };

                            let file_name = format!("文字_{}.txt", Local::now().format("%Y%m%d_%H%M%S"));
                            let file_path = storage.get_notes_dir().join(file_name);
                            let _ = std::fs::write(&file_path, trimmed);

                            let card = Card {
                                id: format!("note-{}", uuid::Uuid::new_v4()),
                                card_type: CardType::Note,
                                title,
                                content: trimmed.to_string(),
                                thumbnail_data: None,
                                meta: Some(format!("{} 字符", trimmed.chars().count())),
                                pinned: false,
                                created_at: now,
                                file_path: Some(file_path.to_string_lossy().to_string()),
                                todo_data: None,
                            };

                            storage.add_card(card.clone());
                            crate::mouse_tracker::trigger_preview(&app_handle, 3500);
                            let _ = app_handle.emit("card_added", &card);
                        }
                    }
                }
            }

            // 2. Check for image
            if settings.watch_clipboard_image {
                if let Ok(img) = clipboard.get_image() {
                    let hash = img.bytes.iter().take(256).fold(0u64, |acc, &b| acc.wrapping_mul(31).wrapping_add(b as u64));
                    if hash != 0 && hash != last_image_hash {
                        last_image_hash = hash;

                        let now = Local::now().timestamp_millis();
                        let file_name = format!("剪贴板_{}.png", Local::now().format("%Y%m%d_%H%M%S"));
                        let target_path = storage.get_screenshots_dir().join(&file_name);

                        if let Some(rgba) = image::RgbaImage::from_raw(
                            img.width as u32,
                            img.height as u32,
                            img.bytes.into_owned(),
                        ) {
                            if rgba.save_with_format(&target_path, image::ImageFormat::Png).is_ok() {
                                let size_bytes = std::fs::metadata(&target_path)
                                    .map(|m| m.len() as usize)
                                    .unwrap_or(0);

                                // Use full original image directly for 100% full-resolution, ultra-crisp display
                                let original_bytes = std::fs::read(&target_path).unwrap_or_default();
                                use base64::Engine;
                                let b64 = base64::engine::general_purpose::STANDARD.encode(&original_bytes);
                                let thumb_data = format!("data:image/png;base64,{}", b64);

                                let card = Card {
                                    id: format!("img-{}", uuid::Uuid::new_v4()),
                                    card_type: CardType::Image,
                                    title: "剪贴板图片".to_string(),
                                    content: target_path.to_string_lossy().to_string(),
                                    thumbnail_data: Some(thumb_data),
                                    meta: Some(format!("{}x{} · {} KB", img.width, img.height, size_bytes / 1024)),
                                    pinned: false,
                                    created_at: now,
                                    file_path: Some(target_path.to_string_lossy().to_string()),
                                    todo_data: None,
                                };

                                storage.add_card(card.clone());
                                crate::mouse_tracker::trigger_preview(&app_handle, 3500);
                                let _ = app_handle.emit("card_added", &card);
                            }
                        }
                    }
                }
            }
        }
    });
}

use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use dirs::data_dir;
use crate::models::{AppSettings, Card, CardType, TodoCard, TodoItem};
use chrono::Local;

pub struct StorageManager {
    base_dir: PathBuf,
    pub settings: Mutex<AppSettings>,
    pub cards: Mutex<Vec<Card>>,
    pub undo_stack: Mutex<Vec<Card>>,
}

impl StorageManager {
    pub fn new() -> Arc<Self> {
        let app_data = data_dir().unwrap_or_else(|| PathBuf::from("."));
        let base_dir = app_data.join("tendedero-rs");
        
        // Ensure directories exist
        let _ = fs::create_dir_all(base_dir.join("Screenshots"));
        let _ = fs::create_dir_all(base_dir.join("Notes"));
        let _ = fs::create_dir_all(base_dir.join("Todos"));
        let _ = fs::create_dir_all(base_dir.join("Trash"));

        let settings = Self::load_settings(&base_dir);
        let cards = Self::load_cards(&base_dir);

        Arc::new(Self {
            base_dir,
            settings: Mutex::new(settings),
            cards: Mutex::new(cards),
            undo_stack: Mutex::new(Vec::new()),
        })
    }

    pub fn get_base_dir(&self) -> PathBuf {
        self.base_dir.clone()
    }

    pub fn get_screenshots_dir(&self) -> PathBuf {
        self.base_dir.join("Screenshots")
    }

    pub fn get_notes_dir(&self) -> PathBuf {
        self.base_dir.join("Notes")
    }

    pub fn get_todos_dir(&self) -> PathBuf {
        self.base_dir.join("Todos")
    }

    pub fn get_trash_dir(&self) -> PathBuf {
        self.base_dir.join("Trash")
    }

    fn load_settings(base_dir: &Path) -> AppSettings {
        let file_path = base_dir.join("settings.json");
        if let Ok(mut file) = File::open(&file_path) {
            let mut content = String::new();
            if file.read_to_string(&mut content).is_ok() {
                if let Ok(settings) = serde_json::from_str(&content) {
                    return settings;
                }
            }
        }
        AppSettings::default()
    }

    pub fn save_settings(&self) {
        let settings = self.settings.lock().unwrap();
        let file_path = self.base_dir.join("settings.json");
        if let Ok(json) = serde_json::to_string_pretty(&*settings) {
            let _ = fs::write(file_path, json);
        }
    }

    fn load_cards(base_dir: &Path) -> Vec<Card> {
        let file_path = base_dir.join("cards.json");
        if let Ok(mut file) = File::open(&file_path) {
            let mut content = String::new();
            if file.read_to_string(&mut content).is_ok() {
                if let Ok(cards) = serde_json::from_str(&content) {
                    return cards;
                }
            }
        }
        // Initialize with a welcome card and a sample TODO sticky note if empty!
        let welcome_cards = Self::initial_cards();
        welcome_cards
    }

    fn initial_cards() -> Vec<Card> {
        let now = Local::now().timestamp_millis();
        
        let sample_todo = Card {
            id: format!("todo-{}", uuid::Uuid::new_v4()),
            card_type: CardType::Todo,
            title: "🎯 今日待办便签".to_string(),
            content: "Rust 版全新上线".to_string(),
            thumbnail_data: None,
            meta: Some("1/3 已完成".to_string()),
            pinned: true,
            created_at: now,
            file_path: None,
            todo_data: Some(TodoCard {
                id: format!("todo-{}", uuid::Uuid::new_v4()),
                title: "🎯 今日待办便签".to_string(),
                color: "yellow".to_string(),
                items: vec![
                    TodoItem { id: "1".to_string(), text: "升级至超轻量 Rust 架构".to_string(), done: true },
                    TodoItem { id: "2".to_string(), text: "体验全新 TODO 便签功能".to_string(), done: false },
                    TodoItem { id: "3".to_string(), text: "按 Ctrl+Alt+S 体验闪电截屏".to_string(), done: false },
                ],
                created_at: now,
                updated_at: now,
            }),
        };

        let sample_note = Card {
            id: format!("note-{}", uuid::Uuid::new_v4()),
            card_type: CardType::Note,
            title: "欢迎使用 Tendedero (Rust 轻量版)".to_string(),
            content: "快捷键指南：\n• Ctrl+Alt+T: 展开/收起晾衣绳\n• Ctrl+Alt+S: 截屏挂绳\n• Ctrl+Alt+N: 新建待办便签\n• Ctrl+Alt+F: 绳上搜索".to_string(),
            thumbnail_data: None,
            meta: Some("98 字符".to_string()),
            pinned: false,
            created_at: now - 1000,
            file_path: None,
            todo_data: None,
        };

        vec![sample_todo, sample_note]
    }

    pub fn save_cards(&self) {
        let cards = self.cards.lock().unwrap();
        let file_path = self.base_dir.join("cards.json");
        if let Ok(json) = serde_json::to_string_pretty(&*cards) {
            let _ = fs::write(file_path, json);
        }
    }

    pub fn add_card(&self, card: Card) {
        {
            let mut cards = self.cards.lock().unwrap();
            cards.insert(0, card);
            let limit = self.settings.lock().unwrap().keep_on_line;
            if cards.len() > limit {
                cards.truncate(limit);
            }
        }
        self.save_cards();
    }

    pub fn remove_card(&self, id: &str) -> Option<Card> {
        let mut removed = None;
        {
            let mut cards = self.cards.lock().unwrap();
            if let Some(pos) = cards.iter().position(|c| c.id == id) {
                let card = cards.remove(pos);
                // Move associated file to trash if it exists
                if let Some(ref path_str) = card.file_path {
                    let path = Path::new(path_str);
                    if path.exists() {
                        let file_name = path.file_name().unwrap_or_default();
                        let target = self.get_trash_dir().join(file_name);
                        let _ = fs::rename(path, target);
                    }
                }
                removed = Some(card.clone());
                self.undo_stack.lock().unwrap().push(card);
            }
        }
        if removed.is_some() {
            self.save_cards();
        }
        removed
    }

    pub fn undo_remove(&self) -> Option<Card> {
        let card = self.undo_stack.lock().unwrap().pop()?;
        {
            let mut cards = self.cards.lock().unwrap();
            cards.insert(0, card.clone());
        }
        self.save_cards();
        Some(card)
    }

    pub fn toggle_pin(&self, id: &str) -> Option<bool> {
        let mut new_state = None;
        {
            let mut cards = self.cards.lock().unwrap();
            if let Some(card) = cards.iter_mut().find(|c| c.id == id) {
                card.pinned = !card.pinned;
                new_state = Some(card.pinned);
            }
        }
        if new_state.is_some() {
            self.save_cards();
        }
        new_state
    }

    pub fn update_todo<F>(&self, card_id: &str, updater: F) -> Option<Card>
    where
        F: FnOnce(&mut TodoCard),
    {
        let mut updated_card = None;
        {
            let mut cards = self.cards.lock().unwrap();
            if let Some(card) = cards.iter_mut().find(|c| c.id == card_id) {
                if let Some(ref mut todo) = card.todo_data {
                    updater(todo);
                    todo.updated_at = Local::now().timestamp_millis();
                    card.title = todo.title.clone();
                    let total = todo.items.len();
                    let done = todo.items.iter().filter(|i| i.done).count();
                    card.meta = Some(format!("{}/{} 已完成", done, total));
                    updated_card = Some(card.clone());
                }
            }
        }
        if updated_card.is_some() {
            self.save_cards();
        }
        updated_card
    }
}

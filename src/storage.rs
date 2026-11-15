use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use dirs::data_dir;
use crate::models::{AppSettings, ArchivedCard, Card, CardType, TodoCard, TodoItem};
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
        let _ = fs::create_dir_all(base_dir.join("Archive"));

        let settings = Self::load_settings(&base_dir);
        let cards = Self::load_cards(&base_dir);

        let manager = Arc::new(Self {
            base_dir,
            settings: Mutex::new(settings),
            cards: Mutex::new(cards),
            undo_stack: Mutex::new(Vec::new()),
        });

        // 1. 关机/开机自动取下：开机启动时自动将上一会话未固定的卡片收纳至本地归档
        manager.archive_unpinned_cards();

        // 2. 自动清理机制：超过 3 天的历史归档与临时文件彻底删除
        manager.clean_expired_data();

        // 3. 启动后台定时任务：每隔 1 小时自动扫描清理满 3 天的历史文件
        let timer_manager = manager.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_secs(3600));
            timer_manager.clean_expired_data();
        });

        manager
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

    #[allow(dead_code)]
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
                if let Ok(mut cards) = serde_json::from_str::<Vec<Card>>(&content) {
                    for card in &mut cards {
                        if matches!(card.card_type, CardType::Screenshot | CardType::Image) {
                            if let Some(ref path_str) = card.file_path {
                                let p = Path::new(path_str);
                                if p.exists() {
                                    if let Ok(bytes) = fs::read(p) {
                                        use base64::Engine;
                                        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                                        card.thumbnail_data = Some(format!("data:image/png;base64,{}", b64));
                                    }
                                }
                            }
                        }
                    }
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
        // Keep cards.json small on disk by stripping base64 image data;
        // original files are already permanently saved in Screenshots directory.
        let disk_cards: Vec<Card> = cards
            .iter()
            .map(|c| {
                let mut clone = c.clone();
                if matches!(clone.card_type, CardType::Screenshot | CardType::Image) && clone.file_path.is_some() {
                    clone.thumbnail_data = None;
                }
                clone
            })
            .collect();
        let file_path = self.base_dir.join("cards.json");
        if let Ok(json) = serde_json::to_string_pretty(&disk_cards) {
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

    pub fn get_archive_dir(&self) -> PathBuf {
        self.base_dir.join("Archive")
    }

    /// 关机/退出/开机自动取下：将未用图钉固定的卡片全部从绳上收纳至本地归档
    pub fn archive_unpinned_cards(&self) -> usize {
        let mut to_archive = Vec::new();
        let now = Local::now().timestamp_millis();
        {
            let mut cards = self.cards.lock().unwrap();
            let mut retained = Vec::new();
            for card in cards.drain(..) {
                if card.pinned {
                    retained.push(card);
                } else {
                    to_archive.push(ArchivedCard {
                        card,
                        archived_at: now,
                    });
                }
            }
            *cards = retained;
        }

        if to_archive.is_empty() {
            return 0;
        }

        let count = to_archive.len();
        let archive_dir = self.get_archive_dir();
        let _ = fs::create_dir_all(&archive_dir);
        let archive_file = archive_dir.join("archive.json");

        // 1. 读取既有归档
        let mut all_archive: Vec<ArchivedCard> = Vec::new();
        if let Ok(mut f) = File::open(&archive_file) {
            let mut s = String::new();
            if f.read_to_string(&mut s).is_ok() {
                if let Ok(parsed) = serde_json::from_str(&s) {
                    all_archive = parsed;
                }
            }
        }

        // 2. 插入新取下的卡片
        for item in to_archive.into_iter().rev() {
            all_archive.insert(0, item);
        }

        // 3. 保存 archive.json (剥离冗余 base64 图片数据减小体积)
        let disk_archive: Vec<ArchivedCard> = all_archive
            .iter()
            .map(|a| {
                let mut clone = a.clone();
                if matches!(clone.card.card_type, CardType::Screenshot | CardType::Image) && clone.card.file_path.is_some() {
                    clone.card.thumbnail_data = None;
                }
                clone
            })
            .collect();
        if let Ok(json) = serde_json::to_string_pretty(&disk_archive) {
            let _ = fs::write(&archive_file, json);
        }

        // 4. 追加至方便用户直接查阅的文本文件：Archive/便签与文字历史归档.txt
        let text_file = archive_dir.join("便签与文字历史归档.txt");
        let date_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let mut txt = String::new();
        for item in &all_archive[0..count] {
            txt.push_str(&format!(
                "\n========================================\n【收纳时间】{}\n【标题】{}\n",
                date_str, item.card.title
            ));
            if let Some(ref todo) = item.card.todo_data {
                txt.push_str("【待办条目】\n");
                for it in &todo.items {
                    txt.push_str(&format!("  [{}] {}\n", if it.done { "√" } else { " " }, it.text));
                }
            } else if !item.card.content.is_empty() {
                txt.push_str(&format!("【内容】{}\n", item.card.content));
            }
            if let Some(ref path) = item.card.file_path {
                txt.push_str(&format!("【关联文件】{}\n", path));
            }
        }
        if !txt.is_empty() {
            use std::io::Write;
            if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(&text_file) {
                let _ = f.write_all(txt.as_bytes());
            }
        }

        // 5. 更新保存绳上的 cards.json
        self.save_cards();

        count
    }

    /// 自动清理：超过 3 天（72小时）的历史归档与关联截图物理文件彻底删除
    pub fn clean_expired_data(&self) -> usize {
        let now = Local::now().timestamp_millis();
        let retention_ms = 3 * 24 * 3600 * 1000; // 3 天有效保留期
        let archive_dir = self.get_archive_dir();
        let archive_file = archive_dir.join("archive.json");

        let mut all_archive: Vec<ArchivedCard> = Vec::new();
        if let Ok(mut f) = File::open(&archive_file) {
            let mut s = String::new();
            if f.read_to_string(&mut s).is_ok() {
                if let Ok(parsed) = serde_json::from_str(&s) {
                    all_archive = parsed;
                }
            }
        }

        let mut kept = Vec::new();
        let mut deleted_count = 0;

        for item in all_archive {
            let age = now.saturating_sub(item.archived_at);
            if age > retention_ms {
                // 满 3 天彻底从磁盘中删除原图文件
                if let Some(ref path_str) = item.card.file_path {
                    let p = Path::new(path_str);
                    if p.exists() {
                        let _ = fs::remove_file(p);
                    }
                }
                deleted_count += 1;
            } else {
                kept.push(item);
            }
        }

        if deleted_count > 0 {
            if let Ok(json) = serde_json::to_string_pretty(&kept) {
                let _ = fs::write(&archive_file, json);
            }
        }

        // 清理回收站 (Trash) 中超过 3 天的文件
        let trash_dir = self.get_trash_dir();
        if let Ok(entries) = fs::read_dir(&trash_dir) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    if let Ok(modified) = meta.modified() {
                        if let Ok(elapsed) = modified.elapsed() {
                            if elapsed.as_secs() > 3 * 24 * 3600 {
                                let _ = fs::remove_file(entry.path());
                            }
                        }
                    }
                }
            }
        }

        deleted_count
    }

    /// 获取所有在本地保存的归档卡片列表（包含 1 天内及 3 天有效期的）
    pub fn get_archived_cards(&self) -> Vec<ArchivedCard> {
        let archive_file = self.get_archive_dir().join("archive.json");
        if let Ok(mut f) = File::open(&archive_file) {
            let mut s = String::new();
            if f.read_to_string(&mut s).is_ok() {
                if let Ok(parsed) = serde_json::from_str(&s) {
                    return parsed;
                }
            }
        }
        Vec::new()
    }
}

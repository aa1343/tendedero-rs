use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum CardType {
    Screenshot,
    Image,
    Note,
    Link,
    Todo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoItem {
    pub id: String,
    pub text: String,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoCard {
    pub id: String,
    pub title: String,
    pub color: String, // "yellow", "green", "blue", "pink", "purple"
    pub items: Vec<TodoItem>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    pub id: String,
    pub card_type: CardType,
    pub title: String,
    pub content: String, // text content, url, or image file path
    pub thumbnail_data: Option<String>, // base64 data url for images
    pub meta: Option<String>, // "1920x1080 · 245 KB" or URL domain or TODO stats
    pub pinned: bool,
    pub created_at: i64,
    pub file_path: Option<String>,
    pub todo_data: Option<TodoCard>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub lang: String,
    pub watch_clipboard_image: bool,
    pub watch_clipboard_text: bool,
    pub watch_clipboard_link: bool,
    pub keep_on_line: usize,
    pub autoclean_days: u32,
    pub sound_enabled: bool,
    pub opens_from_top_edge: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            lang: "zh".to_string(),
            watch_clipboard_image: true,
            watch_clipboard_text: true,
            watch_clipboard_link: true,
            keep_on_line: 25,
            autoclean_days: 3,
            sound_enabled: true,
            opens_from_top_edge: true,
        }
    }
}

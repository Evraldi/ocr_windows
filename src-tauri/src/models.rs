use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanResult {
    pub kind: String,
    pub content: String,
    pub format: String,
    pub preview: Option<String>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct HistoryEntry {
    pub id: i64,
    pub created_at: i64,
    pub kind: String,
    pub content: String,
    pub format: String,
    pub preview: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppInfo {
    pub version: String,
    pub shortcut: String,
    pub ocr_ready: bool,
    pub ocr_error: Option<String>,
}

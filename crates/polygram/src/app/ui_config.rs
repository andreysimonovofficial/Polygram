use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Настройки отображения UI, которые сохраняются в polygram_ui_config.json рядом с экзешником.
#[derive(Default, Serialize, Deserialize)]
pub struct UiConfig {
    pub show_sidebar: bool,
    pub show_command_log_panel: bool,
    pub show_frontend_log_panel: bool,
    pub show_chat_panel: bool,
    pub show_connectors_panel: bool,
    pub show_grid: bool,
    pub show_cursor_coords: bool,
}

impl UiConfig {
    pub fn path() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
            .join("polygram_ui_config.json")
    }

    pub fn load() -> Self {
        let path = Self::path();
        match std::fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) {
        let path = Self::path();
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, json);
        }
    }
}

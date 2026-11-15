use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use polygram_canvas::GridMode;

/// Директория данных приложения (%LOCALAPPDATA%\Polygram\ на Windows).
fn app_data_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        std::env::var("LOCALAPPDATA")
            .ok()
            .map(|p| PathBuf::from(p).join("Polygram"))
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .unwrap_or_default()
                    .join("polygram_data")
            })
    } else {
        std::env::var("HOME")
            .ok()
            .map(|p| PathBuf::from(p).join(".config").join("polygram"))
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .unwrap_or_default()
                    .join("polygram_data")
            })
    }
}

/// Настройки отображения UI, которые сохраняются в polygram_ui_config.json.
#[derive(Serialize, Deserialize)]
pub struct UiConfig {
    pub show_sidebar: bool,
    pub show_command_log_panel: bool,
    pub show_frontend_log_panel: bool,
    pub show_chat_panel: bool,
    pub show_connectors_panel: bool,
    #[serde(default)]
    pub grid_mode: GridMode,
    #[serde(default = "default_grid_color")]
    pub grid_color: [u8; 3],
    pub show_cursor_coords: bool,
    /// Последний открытый файл проекта.
    pub last_opened_file: Option<String>,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            show_sidebar: false,
            show_command_log_panel: false,
            show_frontend_log_panel: false,
            show_chat_panel: false,
            show_connectors_panel: false,
            grid_mode: GridMode::default(),
            grid_color: default_grid_color(),
            show_cursor_coords: false,
            last_opened_file: None,
        }
    }
}

/// Цвет сетки по умолчанию (как в Theme::default).
fn default_grid_color() -> [u8; 3] {
    [35, 35, 42]
}

impl UiConfig {
    pub fn path() -> PathBuf {
        let _ = std::fs::create_dir_all(&app_data_dir());
        app_data_dir().join("polygram_ui_config.json")
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

    /// Путь к последнему открытому файлу, если он существует.
    pub fn last_opened_file_path(&self) -> Option<PathBuf> {
        self.last_opened_file.as_ref().filter(|p| Path::new(p).exists()).cloned().map(PathBuf::from)
    }
}

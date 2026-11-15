use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use polygram_canvas::theme::color_array;
use polygram_canvas::{GridMode, Theme};

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

/// Серийная копия `Theme` для хранения на диске.
/// Все цвета хранятся как [u8; 4] (RGBA), отступы и размеры — как f32 / [f32; 2].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThemeConfig {
    // --- Холст (canvas) ---
    pub bg: [u8; 4],
    pub card_bg: [u8; 4],
    pub card_border: [u8; 4],
    pub card_header: [u8; 4],
    pub line: [u8; 4],
    pub line_hover: [u8; 4],
    pub grid_color: [u8; 4],

    // --- Статусы ---
    pub status_ok: [u8; 4],
    pub status_warning: [u8; 4],
    pub status_deficit: [u8; 4],
    pub status_surplus: [u8; 4],
    pub status_unconnected: [u8; 4],
    pub status_invalid: [u8; 4],

    // --- Текст ---
    pub text: [u8; 4],
    pub text_muted: [u8; 4],

    // --- Оболочка (shell) ---
    pub panel_bg: [u8; 4],
    pub panel_border: [u8; 4],
    pub panel_text: [u8; 4],
    pub menu_bg: [u8; 4],
    pub menu_text: [u8; 4],
    pub menu_hover: [u8; 4],
    pub status_bar_bg: [u8; 4],
    pub status_bar_text: [u8; 4],

    // --- Акцентные цвета ---
    pub accent: [u8; 4],
    pub accent_secondary: [u8; 4],
    pub highlight: [u8; 4],

    // --- Дополнительные цвета egui ---
    pub faint_bg_color: [u8; 4],
    pub extreme_bg_color: [u8; 4],
    pub code_bg_color: [u8; 4],
    pub link_color: [u8; 4],

    // --- Геометрия объектов ---
    pub object_corner_radius: f32,
    pub object_border_width: f32,
    pub object_border_hover_width: f32,
    pub object_header_height_cells: f32,
    pub container_header_height_cells: f32,
    pub port_radius: f32,
    pub text_size: f32,

    // --- Геометрия линий ---
    pub edge_width: f32,
    /// Скруглённые углы ортогональных линий связи.
    pub line_corner_radius: f32,

    // --- Отступы и интервалы (egui::Spacing) ---
    pub item_spacing: [f32; 2],
    pub window_margin: [f32; 2],
    pub menu_margin: [f32; 2],
    pub indent: f32,
    pub combo_height: f32,
    pub slider_width: f32,
    pub resize_corner_size: f32,

    // --- Скругление ---
    pub window_corner_radius: f32,
    pub widget_rounding: f32,

    // --- Шрифты / текстовые стили ---
    pub text_size_body: f32,
    pub text_size_button: f32,
    pub text_size_small: f32,
    pub text_size_heading: f32,
    pub text_size_monospace: f32,

    // --- Тень и свечение ---
    pub shadow_offset: [f32; 2],
    pub shadow_blur: f32,
    pub shadow_color: [u8; 4],
    pub shadow_enabled: bool,
    pub glow_color: [u8; 4],
    pub glow_size: f32,
    pub glow_enabled: bool,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self::from_theme(&Theme::default())
    }
}

impl ThemeConfig {
    /// Собрать серийную копию из текущей темы.
    pub fn from_theme(theme: &Theme) -> Self {
        Self {
            bg: color_array::to_array(theme.bg),
            card_bg: color_array::to_array(theme.card_bg),
            card_border: color_array::to_array(theme.card_border),
            card_header: color_array::to_array(theme.card_header),
            line: color_array::to_array(theme.line),
            line_hover: color_array::to_array(theme.line_hover),
            grid_color: color_array::to_array(theme.grid_color),

            status_ok: color_array::to_array(theme.status_ok),
            status_warning: color_array::to_array(theme.status_warning),
            status_deficit: color_array::to_array(theme.status_deficit),
            status_surplus: color_array::to_array(theme.status_surplus),
            status_unconnected: color_array::to_array(theme.status_unconnected),
            status_invalid: color_array::to_array(theme.status_invalid),

            text: color_array::to_array(theme.text),
            text_muted: color_array::to_array(theme.text_muted),

            panel_bg: color_array::to_array(theme.panel_bg),
            panel_border: color_array::to_array(theme.panel_border),
            panel_text: color_array::to_array(theme.panel_text),
            menu_bg: color_array::to_array(theme.menu_bg),
            menu_text: color_array::to_array(theme.menu_text),
            menu_hover: color_array::to_array(theme.menu_hover),
            status_bar_bg: color_array::to_array(theme.status_bar_bg),
            status_bar_text: color_array::to_array(theme.status_bar_text),

            accent: color_array::to_array(theme.accent),
            accent_secondary: color_array::to_array(theme.accent_secondary),
            highlight: color_array::to_array(theme.highlight),

            faint_bg_color: color_array::to_array(theme.faint_bg_color),
            extreme_bg_color: color_array::to_array(theme.extreme_bg_color),
            code_bg_color: color_array::to_array(theme.code_bg_color),
            link_color: color_array::to_array(theme.link_color),

            object_corner_radius: theme.object_corner_radius,
            object_border_width: theme.object_border_width,
            object_border_hover_width: theme.object_border_hover_width,
            object_header_height_cells: theme.object_header_height_cells,
            container_header_height_cells: theme.container_header_height_cells,
            port_radius: theme.port_radius,
            text_size: theme.text_size,
            edge_width: theme.edge_width,
            line_corner_radius: theme.line_corner_radius,

            item_spacing: theme.item_spacing,
            window_margin: theme.window_margin,
            menu_margin: theme.menu_margin,
            indent: theme.indent,
            combo_height: theme.combo_height,
            slider_width: theme.slider_width,
            resize_corner_size: theme.resize_corner_size,

            window_corner_radius: theme.window_corner_radius,
            widget_rounding: theme.widget_rounding,

            text_size_body: theme.text_size_body,
            text_size_button: theme.text_size_button,
            text_size_small: theme.text_size_small,
            text_size_heading: theme.text_size_heading,
            text_size_monospace: theme.text_size_monospace,

            shadow_offset: theme.shadow_offset,
            shadow_blur: theme.shadow_blur,
            shadow_color: color_array::to_array(theme.shadow_color),
            shadow_enabled: theme.shadow_enabled,
            glow_color: color_array::to_array(theme.glow_color),
            glow_size: theme.glow_size,
            glow_enabled: theme.glow_enabled,
        }
    }

    /// Применить серийную копию к теме (вернуть `Theme`).
    pub fn into_theme(self) -> Theme {
        Theme {
            bg: color_array::from_array(self.bg),
            card_bg: color_array::from_array(self.card_bg),
            card_border: color_array::from_array(self.card_border),
            card_header: color_array::from_array(self.card_header),
            line: color_array::from_array(self.line),
            line_hover: color_array::from_array(self.line_hover),
            grid_color: color_array::from_array(self.grid_color),

            status_ok: color_array::from_array(self.status_ok),
            status_warning: color_array::from_array(self.status_warning),
            status_deficit: color_array::from_array(self.status_deficit),
            status_surplus: color_array::from_array(self.status_surplus),
            status_unconnected: color_array::from_array(self.status_unconnected),
            status_invalid: color_array::from_array(self.status_invalid),

            text: color_array::from_array(self.text),
            text_muted: color_array::from_array(self.text_muted),

            panel_bg: color_array::from_array(self.panel_bg),
            panel_border: color_array::from_array(self.panel_border),
            panel_text: color_array::from_array(self.panel_text),
            menu_bg: color_array::from_array(self.menu_bg),
            menu_text: color_array::from_array(self.menu_text),
            menu_hover: color_array::from_array(self.menu_hover),
            status_bar_bg: color_array::from_array(self.status_bar_bg),
            status_bar_text: color_array::from_array(self.status_bar_text),

            accent: color_array::from_array(self.accent),
            accent_secondary: color_array::from_array(self.accent_secondary),
            highlight: color_array::from_array(self.highlight),

            faint_bg_color: color_array::from_array(self.faint_bg_color),
            extreme_bg_color: color_array::from_array(self.extreme_bg_color),
            code_bg_color: color_array::from_array(self.code_bg_color),
            link_color: color_array::from_array(self.link_color),

            object_corner_radius: self.object_corner_radius,
            object_border_width: self.object_border_width,
            object_border_hover_width: self.object_border_hover_width,
            object_header_height_cells: self.object_header_height_cells,
            container_header_height_cells: self.container_header_height_cells,
            port_radius: self.port_radius,
            text_size: self.text_size,
            edge_width: self.edge_width,
            line_corner_radius: self.line_corner_radius,

            item_spacing: self.item_spacing,
            window_margin: self.window_margin,
            menu_margin: self.menu_margin,
            indent: self.indent,
            combo_height: self.combo_height,
            slider_width: self.slider_width,
            resize_corner_size: self.resize_corner_size,

            window_corner_radius: self.window_corner_radius,
            widget_rounding: self.widget_rounding,

            text_size_body: self.text_size_body,
            text_size_button: self.text_size_button,
            text_size_small: self.text_size_small,
            text_size_heading: self.text_size_heading,
            text_size_monospace: self.text_size_monospace,

            shadow_offset: self.shadow_offset,
            shadow_blur: self.shadow_blur,
            shadow_color: color_array::from_array(self.shadow_color),
            shadow_enabled: self.shadow_enabled,
            glow_color: color_array::from_array(self.glow_color),
            glow_size: self.glow_size,
            glow_enabled: self.glow_enabled,
        }
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
    /// Тема оформления (цвета, отступы, скругление, шрифты).
    #[serde(default)]
    pub theme: ThemeConfig,
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
            theme: ThemeConfig::default(),
            last_opened_file: None,
        }
    }
}

/// Цвет сетки по умолчанию (R49 G49 B58).
fn default_grid_color() -> [u8; 3] {
    [49, 49, 58]
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

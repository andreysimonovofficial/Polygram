use egui::Color32;

/// Единая тёмная тема приложения Polygram.
/// Источник истины для всех цветов: холст (canvas) и оболочка (polygram shell).
/// При появлении DesignDocument.theme в polygram-core — значения берутся оттуда.
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    // --- Холст (canvas) ---
    pub bg: Color32,
    pub card_bg: Color32,
    pub card_border: Color32,
    pub card_header: Color32,
    pub line: Color32,
    pub line_hover: Color32,
    pub grid_color: Color32,

    // --- Статусы (общие для холста и оболочки) ---
    pub status_ok: Color32,
    pub status_warning: Color32,
    pub status_deficit: Color32,
    pub status_surplus: Color32,
    pub status_unconnected: Color32,
    pub status_invalid: Color32,

    // --- Текст (общие) ---
    pub text: Color32,
    pub text_muted: Color32,

    // --- Оболочка (shell): панели, меню, строка статуса ---
    pub panel_bg: Color32,
    pub panel_border: Color32,
    pub panel_text: Color32,
    pub menu_bg: Color32,
    pub menu_text: Color32,
    pub menu_hover: Color32,
    pub status_bar_bg: Color32,
    pub status_bar_text: Color32,

    // --- Акцентные цвета ---
    pub accent: Color32,
    pub accent_secondary: Color32,
    pub highlight: Color32,

    // --- Геометрия объектов (из ObjectStyle) ---
    pub object_corner_radius: f32,
    pub object_border_width: f32,
    pub object_border_hover_width: f32,
    pub object_header_height_cells: f32,
    pub container_header_height_cells: f32,
    pub port_radius: f32,
    pub text_size: f32,

    // --- Геометрия линий ---
    pub edge_width: f32,

    // --- Тень и свечение ---
    pub shadow_offset: [f32; 2],
    pub shadow_blur: f32,
    pub shadow_color: Color32,
    pub shadow_enabled: bool,
    pub glow_color: Color32,
    pub glow_size: f32,
    pub glow_enabled: bool,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            // Холст
            bg: Color32::from_rgb(18, 18, 22),
            card_bg: Color32::from_rgb(28, 28, 34),
            card_border: Color32::from_rgb(60, 60, 72),
            card_header: Color32::from_rgb(45, 45, 54),
            line: Color32::from_rgb(120, 120, 135),
            line_hover: Color32::from_rgb(160, 160, 180),
            grid_color: Color32::from_rgb(35, 35, 42),

            // Статусы
            status_ok: Color32::from_rgb(76, 175, 80),
            status_warning: Color32::from_rgb(255, 152, 0),
            status_deficit: Color32::from_rgb(244, 67, 54),
            status_surplus: Color32::from_rgb(3, 169, 244),
            status_unconnected: Color32::from_rgb(100, 100, 110),
            status_invalid: Color32::from_rgb(158, 158, 158),

            // Текст
            text: Color32::from_rgb(230, 230, 235),
            text_muted: Color32::from_rgb(150, 150, 160),

            // Оболочка
            panel_bg: Color32::from_rgb(28, 28, 34),
            panel_border: Color32::from_rgb(60, 60, 72),
            panel_text: Color32::from_rgb(230, 230, 235),
            menu_bg: Color32::from_rgb(28, 28, 34),
            menu_text: Color32::from_rgb(230, 230, 235),
            menu_hover: Color32::from_rgb(60, 60, 72),
            status_bar_bg: Color32::from_rgb(28, 28, 34),
            status_bar_text: Color32::from_rgb(230, 230, 235),

            // Акценты
            accent: Color32::from_rgb(0, 217, 255),
            accent_secondary: Color32::from_rgb(100, 180, 255),
            highlight: Color32::from_rgb(200, 200, 0),

            // Геометрия объектов
            object_corner_radius: 2.0,
            object_border_width: 1.0,
            object_border_hover_width: 2.0,
            object_header_height_cells: 1.0,
            container_header_height_cells: 0.5,
            port_radius: 3.0,
            text_size: 14.0,
            edge_width: 1.5,

            // Тень и свечение
            shadow_offset: [4.0, 4.0],
            shadow_blur: 8.0,
            shadow_color: Color32::from_rgba_unmultiplied(0, 0, 0, 80),
            shadow_enabled: false,
            glow_color: Color32::from_rgba_unmultiplied(100, 100, 200, 60),
            glow_size: 6.0,
            glow_enabled: false,
        }
    }
}

use egui::Color32;
use polygram_core::types::StatusCode;

/// Единая тёмная тема приложения Polygram.
/// Источник истины для всех цветов и геометрии: холст (canvas) и оболочка (polygram shell).
/// При появлении DesignDocument.theme в polygram-core — значения берутся оттуда.
///
/// Все поля копируются (Copy), чтобы легко передавать по значению в функции рисования.
/// Сериализуемая копия для хранения на диске находится в `polygram::app::ui_config::ThemeConfig`.
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    // ========== Холст (canvas) ==========
    pub bg: Color32,
    pub card_bg: Color32,
    pub card_border: Color32,
    pub card_header: Color32,
    pub line: Color32,
    pub line_hover: Color32,
    pub grid_color: Color32,

    // ========== Статусы (общие для холста и оболочки) ==========
    pub status_ok: Color32,
    pub status_warning: Color32,
    pub status_deficit: Color32,
    pub status_surplus: Color32,

    // ========== Заливка фона отдела по его агрегированному статусу ==========
    // Цвета темнее цвета линий связей (line: 120,120,135), чтобы линии оставались различимы.
    pub status_fill_ok: Color32,
    pub status_fill_surplus: Color32,
    pub status_fill_inprogress: Color32,
    pub status_fill_expected: Color32,

    // ========== Текст (общие) ==========
    pub text: Color32,
    pub text_muted: Color32,

    // ========== Оболочка (shell): панели, меню, строка статуса ==========
    pub panel_bg: Color32,
    pub panel_border: Color32,
    pub panel_text: Color32,
    pub menu_bg: Color32,
    pub menu_text: Color32,
    pub menu_hover: Color32,
    pub status_bar_bg: Color32,
    pub status_bar_text: Color32,

    // ========== Акцентные цвета ==========
    pub accent: Color32,
    pub accent_secondary: Color32,
    pub highlight: Color32,

    // ========== Дополнительные цвета egui ==========
    pub faint_bg_color: Color32,
    pub extreme_bg_color: Color32,
    pub code_bg_color: Color32,
    pub link_color: Color32,

    // ========== Геометрия объектов (из ObjectStyle) ==========
    pub object_corner_radius: f32,      // скругление краёв карточек (отделы, процессы, отделы)
    pub object_border_width: f32,
    pub object_border_hover_width: f32,
    pub object_header_height_cells: f32,
    pub container_header_height_cells: f32,
    pub port_radius: f32,
    pub text_size: f32,

    // ========== Геометрия линий ==========
    pub edge_width: f32,
    /// Скруглённые углы ортогональных линий связи (0 = прямые углы, как сейчас).
    pub line_corner_radius: f32,

    // ========== Отступы и интервалы (egui::Spacing) ==========
    /// Расстояние между соседними виджетами [x, y].
    pub item_spacing: [f32; 2],
    /// Внешний отступ окна [x, y].
    pub window_margin: [f32; 2],
    /// Отступ внутри меню [x, y].
    pub menu_margin: [f32; 2],
    /// Отступ для иерархических виджетов (дерево, collapsing).
    pub indent: f32,
    /// Высота выпадающих списков.
    pub combo_height: f32,
    /// Ширина ползунка.
    pub slider_width: f32,
    /// Размер маркера изменения размера угла.
    pub resize_corner_size: f32,

    // ========== Скругление (egui visuals) ==========
    /// Скругление углов окон и панелей.
    pub window_corner_radius: f32,
    /// Скругление углов виджетов (кнопки, поля ввода, фреймы).
    pub widget_rounding: f32,

    // ========== Шрифты / текстовые стили egui (размеры в пикселях) ==========
    pub text_size_body: f32,
    pub text_size_button: f32,
    pub text_size_small: f32,
    pub text_size_heading: f32,
    pub text_size_monospace: f32,

    // ========== Тень и свечение ==========
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
            grid_color: Color32::from_rgb(49, 49, 58),

            // Статусы
            status_ok: Color32::from_rgb(76, 175, 80),
            status_warning: Color32::from_rgb(255, 152, 0),
            status_deficit: Color32::from_rgb(244, 67, 54),
            status_surplus: Color32::from_rgb(3, 169, 244),

            // Заливка фона отдела (темнее линий 120,120,135)
            status_fill_ok: Color32::from_rgb(15, 80, 25),
            status_fill_surplus: Color32::from_rgb(90, 70, 0),
            status_fill_inprogress: Color32::from_rgb(100, 45, 0),
            status_fill_expected: Color32::from_rgb(90, 15, 15),

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

            // Дополнительные цвета egui
            faint_bg_color: Color32::from_rgb(38, 38, 44),
            extreme_bg_color: Color32::from_rgb(10, 10, 12),
            code_bg_color: Color32::from_rgb(30, 30, 36),
            link_color: Color32::from_rgb(100, 180, 255),

            // Геометрия объектов
            object_corner_radius: 2.0,
            object_border_width: 1.0,
            object_border_hover_width: 2.0,
            object_header_height_cells: 1.0,
            container_header_height_cells: 0.5,
            port_radius: 3.0,
            text_size: 14.0,
            edge_width: 1.5,
            line_corner_radius: 0.0,

            // Отступы и интервалы (egui::Spacing по умолчанию)
            item_spacing: [8.0, 8.0],
            window_margin: [8.0, 8.0],
            menu_margin: [8.0, 8.0],
            indent: 16.0,
            combo_height: 180.0,
            slider_width: 140.0,
            resize_corner_size: 12.0,

            // Скругление
            window_corner_radius: 4.0,
            widget_rounding: 2.0,

            // Шрифты / текстовые стили
            text_size_body: 14.0,
            text_size_button: 14.0,
            text_size_small: 12.0,
            text_size_heading: 20.0,
            text_size_monospace: 13.0,

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

impl Theme {
    /// Скругление углов окон/панелей в терминах egui.
    pub fn window_rounding(&self) -> egui::epaint::CornerRadius {
        egui::epaint::CornerRadius::from(self.window_corner_radius)
    }

    /// Скругление углов виджетов в терминах egui.
    pub fn widget_rounding_egui(&self) -> egui::epaint::CornerRadius {
        egui::epaint::CornerRadius::from(self.widget_rounding)
    }

    /// Скругление краёв карточек холста с учётом зума.
    pub fn object_rounding(&self, zoom: f32) -> f32 {
        self.object_corner_radius * zoom
    }

    /// Скругление углов линий связи с учётом зума.
    pub fn line_rounding(&self, zoom: f32) -> f32 {
        self.line_corner_radius * zoom
    }

    /// Заливка фона отдела по агрегированному статусу.
    pub fn status_fill_color(&self, status: StatusCode) -> Color32 {
        match status {
            StatusCode::Ok => self.status_fill_ok,
            StatusCode::Surplus => self.status_fill_surplus,
            StatusCode::InProgress => self.status_fill_inprogress,
            StatusCode::Expected => self.status_fill_expected,
        }
    }
}

/// Вспомогательные преобразования цвета <-> [u8; 4] для серийного хранения.
pub mod color_array {
use egui::Color32;

    pub fn to_array(c: Color32) -> [u8; 4] {
        [c.r(), c.g(), c.b(), c.a()]
    }

    pub fn from_array(a: [u8; 4]) -> Color32 {
        Color32::from_rgba_unmultiplied(a[0], a[1], a[2], a[3])
    }
}

use egui::Color32;

/// Цвета и геометрическая стилистика темы.
/// Цвета берутся из DesignDocument.theme, геометрия — из ObjectStyle выбранного вида объекта.
/// Это позволяет Агенту отрисовать макет в точности, как задан в инструменте дизайна.
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    // Цвета
    pub bg: Color32,
    pub grid: Color32,
    pub card_bg: Color32,
    pub card_border: Color32,
    pub card_header: Color32,
    pub line: Color32,
    pub line_hover: Color32,
    pub status_ok: Color32,
    pub status_warning: Color32,
    pub status_deficit: Color32,
    pub status_surplus: Color32,
    pub status_unconnected: Color32,
    pub status_invalid: Color32,
    pub text: Color32,
    pub text_muted: Color32,

    // Геометрия объектов (из ObjectStyle)
    pub object_corner_radius: f32,
    pub object_border_width: f32,
    pub object_border_hover_width: f32,
    pub object_header_height_cells: f32,
    pub container_header_height_cells: f32,
    pub port_radius: f32,
    pub text_size: f32,

    // Геометрия линий
    pub edge_width: f32,

    // Тень и свечение (из ObjectStyle, опционально)
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
            bg: Color32::from_rgb(18, 18, 22),
            grid: Color32::from_rgb(35, 35, 42),
            card_bg: Color32::from_rgb(28, 28, 34),
            card_border: Color32::from_rgb(60, 60, 72),
            card_header: Color32::from_rgb(45, 45, 54),
            line: Color32::from_rgb(120, 120, 135),
            line_hover: Color32::from_rgb(160, 160, 180),
            status_ok: Color32::from_rgb(76, 175, 80),
            status_warning: Color32::from_rgb(255, 193, 7),
            status_deficit: Color32::from_rgb(244, 67, 54),
            status_surplus: Color32::from_rgb(3, 169, 244),
            status_unconnected: Color32::from_rgb(100, 100, 110),
            status_invalid: Color32::from_rgb(158, 158, 158),
            text: Color32::from_rgb(230, 230, 235),
            text_muted: Color32::from_rgb(150, 150, 160),
            object_corner_radius: 2.0,
            object_border_width: 1.0,
            object_border_hover_width: 2.0,
            object_header_height_cells: 1.0,
            container_header_height_cells: 0.5,
            port_radius: 3.0,
            text_size: 14.0,
            edge_width: 1.5,
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

//! Единая тёмная тема холста Polygram.
//! Источник истины для цветов и геометрии холста.

use egui::{Color32, CornerRadius, Margin, epaint::Shadow};

/// Тёмная тема холста Polygram.
/// Содержит цвета и размеры, используемые при отрисовке фона и точечной сетки.
#[derive(Clone, Copy, Debug, Default)]
pub struct Theme {
    /// Фон холста.
    pub bg: Color32,
    /// Цвет точек сетки.
    pub grid_color: Color32,
    /// Размер одной клетки в мировых координатах.
    pub cell_size: f32,
    /// Радиус точки сетки в долях от размера клетки.
    pub dot_radius_world: f32,
    /// Тень окон и контекстных меню. По умолчанию без тени.
    pub shadow: Shadow,
    /// Скругление углов внешней рамки контекстного меню.
    pub menu_frame_rounding: CornerRadius,
    /// Скругление углов пунктов (кнопок) контекстного меню.
    pub menu_item_rounding: CornerRadius,
    /// Отступ внешней рамки контекстного меню от пунктов внутри.
    pub menu_frame_margin: Margin,
    /// Фон внешней рамки контекстного меню.
    pub menu_frame_fill: Color32,
    /// Цвет контура внешней рамки контекстного меню.
    pub menu_frame_stroke_color: Color32,
    /// Толщина контура внешней рамки контекстного меню.
    pub menu_frame_stroke_width: f32,
    /// Фон пунктов контекстного меню.
    pub menu_item_bg_fill: Color32,

    // ========== Объекты холста (процессы, отделы) ==========
    /// Заливка фона отдела.
    pub object_bg: Color32,
    /// Граница отдела.
    pub object_border: Color32,
    /// Акцентный цвет (рамка выделения, активные элементы).
    pub accent: Color32,
    /// Скругление углов отдела (в клетках).
    pub object_corner_radius: f32,
    /// Толщина границы отдела (в клетках).
    pub object_border_width: f32,
    /// Высота шапки отдела (в клетках).
    pub object_header_height_cells: f32,
    /// Размер текста отдела.
    pub text_size: f32,
    /// Цвет основного текста отдела.
    pub text: Color32,
    /// Цвет приглушённого текста отдела (id).
    pub text_muted: Color32,
}

impl Theme {
    /// Создаёт тёмную тему по умолчанию.
    pub fn dark() -> Self {
        Self {
            bg: Color32::from_rgb(18, 18, 22),
            grid_color: Color32::from_rgb(49, 49, 58),
            cell_size: 16.0,
            dot_radius_world: 0.04,
            shadow: Shadow::NONE,
            menu_frame_rounding: CornerRadius::same(8),
            menu_item_rounding: CornerRadius::same(2),
            menu_frame_margin: Margin::same(4),
            menu_frame_fill: Color32::from_rgb(30, 30, 30),     // #1E1E1E
            menu_frame_stroke_color: Color32::from_rgb(56, 56, 56), // #383838
            menu_frame_stroke_width: 0.5,
            menu_item_bg_fill: Color32::from_rgb(30, 30, 30),   // #1E1E1E

            // ========== Объекты холста (процессы, отделы) ==========
            object_bg: Color32::from_rgb(28, 28, 34),           // #1C1C22
            object_border: Color32::from_rgb(60, 60, 72),       // #3C3C48
            accent: Color32::from_rgb(0, 217, 255),             // #00D9FF
            object_corner_radius: 2.0,
            object_border_width: 1.0,
            object_header_height_cells: 1.0,
            text_size: 14.0,
            text: Color32::from_rgb(230, 230, 235),             // #E6E6EB
            text_muted: Color32::from_rgb(150, 150, 160),       // #9696A0
        }
    }
}

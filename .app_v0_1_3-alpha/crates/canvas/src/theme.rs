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
            menu_frame_rounding: CornerRadius::same(4),
            menu_item_rounding: CornerRadius::same(2),
            menu_frame_margin: Margin::same(4),
            menu_frame_fill: Color32::from_rgb(30, 30, 30),     // #1E1E1E
            menu_frame_stroke_color: Color32::from_rgb(56, 56, 56), // #383838
            menu_frame_stroke_width: 0.5,
            menu_item_bg_fill: Color32::from_rgb(52, 52, 52),   // #343434
        }
    }
}

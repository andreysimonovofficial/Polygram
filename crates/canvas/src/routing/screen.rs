//! Перевод клеток сетки в экранные пиксели и обратно (общий для линий и портов).

use super::model::U;
use egui::{Pos2, Vec2};

/// Размер клетки в мировых пикселях: `cell_px` уже содержит зум камеры.
pub fn cell_size_from_px(cell_px: f32, camera_zoom: f32) -> f32 {
    cell_px / camera_zoom.max(0.001)
}

/// Точка маршрута (субклетки) → экранные пиксели.
pub fn to_screen(p: (i32, i32), cell_size: f32, offset: Vec2, zoom: f32) -> Pos2 {
    Pos2::new(
        (p.0 as f32 / U as f32 * cell_size + offset.x) * zoom,
        (p.1 as f32 / U as f32 * cell_size + offset.y) * zoom,
    )
}

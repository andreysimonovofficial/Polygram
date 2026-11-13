//! Расчёт экранных позиций портов сцепления строк показателей.
//! Чистая геометрия без egui-отрисовки: только координаты точек сцепления.

use egui::{Pos2, Vec2};
use polygram_core::geometry::EntryGeom;
use polygram_core::layout::Rect as CoreRect;
use polygram_core::EntryIx;

/// cell_size из cell_px (который уже содержит зум).
pub fn cell_size_from_px(cell_px: f32, camera_zoom: f32) -> f32 {
    cell_px / camera_zoom.max(0.001)
}

/// Вычислить экранную позицию точки сцепления строки показателя.
/// Логика идентична paint_process: одинаковая высота строки, зазоры между строками.
/// Точка находится внутри тела строки: слева или справа в зависимости от `exit_right`.
/// `is_output` определяет контейнер (запрос — выше, выход — ниже), `exit_right` — сторону X.
pub fn visual_port_screen(
    object_rect: &CoreRect,
    entries: &[EntryGeom],
    target_ix: EntryIx,
    is_output: bool,
    exit_right: bool,
    cell_px: f32,
    camera_offset: Vec2,
    camera_zoom: f32,
) -> Option<Pos2> {
    let k = entries.iter().position(|e| e.ix == target_ix.0)?;
    let y = metric_row_center_y(object_rect, k, is_output, cell_px, camera_offset, camera_zoom);
    let x = metric_port_screen_x(object_rect, exit_right, cell_px, camera_offset, camera_zoom);
    Some(Pos2::new(x, y))
}

/// Чистая функция: экранный Y-центр строки показателя номер k.
pub fn metric_row_center_y(
    object_rect: &CoreRect,
    k: usize,
    is_output: bool,
    cell_px: f32,
    camera_offset: Vec2,
    camera_zoom: f32,
) -> f32 {
    let cell_size = cell_size_from_px(cell_px, camera_zoom);

    let object_min_y = (object_rect.y as f32 * cell_size + camera_offset.y) * camera_zoom;
    let object_h_px = object_rect.h as f32 * cell_px;

    let header_h_px = object_h_px * 0.10;
    let pad = 8.0 * camera_zoom;
    let container_rect_top = object_min_y + header_h_px + pad;
    let container_h = object_h_px - header_h_px - 2.0 * pad;
    let gap = pad;
    let split_h = container_h - gap;
    let container_h_px = split_h * 0.5;

    let container_top = if is_output {
        container_rect_top + container_h_px + gap
    } else {
        container_rect_top
    };

    let container_header_h = container_h_px * 0.20;
    let content_top = container_top + container_header_h;
    let content_h = container_h_px - container_header_h;

    let gap_total = (crate::paint::constants::MAX_VISIBLE_METRICS - 1) as f32
        * crate::paint::constants::ROW_GAP_PX
        * camera_zoom;
    let row_h = if crate::paint::constants::MAX_VISIBLE_METRICS > 0 {
        (content_h - gap_total) / crate::paint::constants::MAX_VISIBLE_METRICS as f32
    } else {
        0.0
    };
    let row_h = row_h.max(crate::paint::constants::MIN_ROW_H_PX * camera_zoom).floor();
    let row_step = row_h + crate::paint::constants::ROW_GAP_PX * camera_zoom;

    let row_y = content_top + k as f32 * row_step;
    row_y + row_h / 2.0
}

/// Чистая функция: экранная X-позиция порта строки показателя.
/// exit_right = true — порт на правой границе объекта, false — на левой.
pub fn metric_port_screen_x(
    object_rect: &CoreRect,
    exit_right: bool,
    cell_px: f32,
    camera_offset: Vec2,
    camera_zoom: f32,
) -> f32 {
    let cell_size = cell_size_from_px(cell_px, camera_zoom);

    let object_min_x = (object_rect.x as f32 * cell_size + camera_offset.x) * camera_zoom;
    let object_w_px = object_rect.w as f32 * cell_px;
    let pad = 8.0 * camera_zoom;
    let dot_r = crate::paint::constants::CONNECTION_DOT_RADIUS * camera_zoom;

    if exit_right {
        let object_max_x = object_min_x + object_w_px - pad;
        object_max_x - dot_r
    } else {
        let object_min_x_adj = object_min_x + pad;
        object_min_x_adj + dot_r
    }
}

//! Расчёт экранных позиций портов сцепления строк показателей.
//! Чистая геометрия без egui-отрисовки: только координаты точек сцепления.

use super::model::Port;
use super::screen::cell_size_from_px;
use egui::{Pos2, Vec2};
use polygram_core::geometry::EntryGeom;
use polygram_core::layout::Rect as CoreRect;
use polygram_core::EntryIx;
use polygram_core::types::NodeKind;

/// Точка сцепления строки показателя в клетках сетки, при зуме 1 и нулевом смещении камеры.
/// Не зависит от камеры, поэтому годится для кэша маршрутов. `exit_right` — сторона карточки.
pub fn port_in_cells(
    object_rect: &CoreRect,
    entries: &[EntryGeom],
    target_ix: EntryIx,
    is_result: bool,
    exit_right: bool,
    cell_size: f32,
    kind: NodeKind,
) -> Option<Port> {
    let p = visual_port_screen(
        object_rect,
        entries,
        target_ix,
        is_result,
        exit_right,
        cell_size,
        Vec2::ZERO,
        1.0,
        kind,
    )?;
    Some(Port {
        x: p.x / cell_size,
        y: p.y / cell_size,
    })
}

/// Вычислить экранную позицию точки сцепления строки показателя.
/// Для Process используется фиксированная CSS-геометрия 256×256px.
/// `is_result` определяет контейнер (запрос — выше, результат — ниже), `exit_right` — сторону X.
pub fn visual_port_screen(
    object_rect: &CoreRect,
    entries: &[EntryGeom],
    target_ix: EntryIx,
    is_result: bool,
    exit_right: bool,
    cell_px: f32,
    camera_offset: Vec2,
    camera_zoom: f32,
    kind: NodeKind,
) -> Option<Pos2> {
    let k = entries.iter().position(|e| e.ix == target_ix.0)?;

    if kind == NodeKind::Process {
        // Фиксированная CSS-геометрия: карточка 256×256px, контейнеры на top:32 и top:144,
        // строки на y:28/48/68/88 внутри контейнера, порты на символах ◀/▶.
        let cell_size = cell_size_from_px(cell_px, camera_zoom);
        let ox = (object_rect.x as f32 * cell_size + camera_offset.x) * camera_zoom;
        let oy = (object_rect.y as f32 * cell_size + camera_offset.y) * camera_zoom;

        let row_offsets: [f32; 4] = [28.0, 48.0, 68.0, 88.0];
        if k >= row_offsets.len() {
            return None;
        }

        let container_top = if is_result { 144.0 } else { 32.0 };
        let y = oy + (container_top + row_offsets[k] + 8.0) * camera_zoom; // +8 = центр строки 16px

        // Порт на символе ▶ (источник связи): Request — справа (x=232), Результат — слева (x=8)
        let x = if exit_right {
            // Правая точка сцепления совпадает с правым треугольником строки:
            // row_x + row_w - 4.0 * zoom = ox + 8 + 240 - 4 = ox + 244.
            ox + 244.0 * camera_zoom
        } else {
            // Левая точка сцепления совпадает с левым треугольником строки:
            // cx + 8.0 * zoom = ox + 4 + 8 = ox + 12.
            ox + 12.0 * camera_zoom
        };

        return Some(Pos2::new(x, y));
    }

    // Существующая логика для остальных типов объектов.
    let y = metric_row_center_y(object_rect, k, is_result, cell_px, camera_offset, camera_zoom);
    let x = metric_port_screen_x(object_rect, exit_right, cell_px, camera_offset, camera_zoom);
    Some(Pos2::new(x, y))
}

/// Чистая функция: экранный Y-центр строки показателя номер k.
pub fn metric_row_center_y(
    object_rect: &CoreRect,
    k: usize,
    is_result: bool,
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

    let container_top = if is_result {
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

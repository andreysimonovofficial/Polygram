use crate::theme::Theme;
use crate::paint_process::MAX_VISIBLE_METRICS;
use egui::{Painter, Pos2, Shape, Stroke, Vec2};
use polygram_core::layout::Rect as CoreRect;
use polygram_core::{DynamicState, EdgeGeom, Geometry, StatusCode};

/// Вычислить экранную позицию порта строки показателя:
/// середина правой границы для ВЫХОДА, середина левой для ЗАПРОСА.
fn entry_row_port_screen(
    object_rect: &CoreRect,
    entries: &[polygram_core::EntryGeom],
    target_ix: polygram_core::EntryIx,
    is_output: bool,
    cell_px: f32,
    camera_offset: Vec2,
    camera_zoom: f32,
) -> Option<Pos2> {
    let k = entries.iter().position(|e| e.ix == target_ix.0)?;

    // Объект в экранных пикселях (как в paint_process)
    let object_min_x = object_rect.x as f32 * cell_px + camera_offset.x * camera_zoom;
    let object_min_y = object_rect.y as f32 * cell_px + camera_offset.y * camera_zoom;
    let object_w_px = object_rect.w as f32 * cell_px;
    let object_h_px = object_rect.h as f32 * cell_px;

    // Те же пропорции, что в paint_process
    let header_h_px = object_h_px * 0.10;
    let pad = 8.0 * camera_zoom;
    let gap = pad;
    let container_h = (object_h_px - header_h_px - 2.0 * pad - gap) * 0.5;

    let container_top = if is_output {
        // ВЫХОД — второй контейнер
        header_h_px + pad + container_h + gap
    } else {
        // ЗАПРОС — первый контейнер
        header_h_px + pad
    };

    let container_header_h = container_h * 0.20;
    let content_top = container_top + container_header_h;
    let content_h = container_h - container_header_h;

    let visible = entries.len().min(MAX_VISIBLE_METRICS);
    let show_add = visible < MAX_VISIBLE_METRICS;
    let total_rows = visible + if show_add { 1 } else { 0 };
    let row_h = if total_rows > 0 { content_h / total_rows as f32 } else { 0.0 };

    let row_y = content_top + k as f32 * row_h;
    let row_screen_y = object_min_y + row_y;
    let row_center_y = row_screen_y + row_h / 2.0;

    if is_output {
        let object_max_x = object_min_x + object_w_px;
        Some(Pos2::new(object_max_x, row_center_y))
    } else {
        Some(Pos2::new(object_min_x, row_center_y))
    }
}

pub fn paint_edge(
    painter: &Painter,
    edge: &EdgeGeom,
    geometry: &Geometry,
    dynamic: &DynamicState,
    theme: &Theme,
    cell_px: f32,
    camera_offset: Vec2,
    camera_zoom: f32,
) {
    let from_obj = geometry
        .objects
        .iter()
        .find(|o| o.outputs.iter().any(|e| e.ix == edge.from));
    let to_obj = geometry
        .objects
        .iter()
        .find(|o| o.requests.iter().any(|e| e.ix == edge.to));

    let line_color = match (from_obj, to_obj) {
        (Some(fo), Some(to)) => {
            let k_from = fo
                .outputs
                .iter()
                .position(|e| e.ix == edge.from)
                .unwrap_or(0);
            let k_to = to
                .requests
                .iter()
                .position(|e| e.ix == edge.to)
                .unwrap_or(0);
            let from_status = dynamic.status[fo.outputs[k_from].ix as usize];
            let to_status = dynamic.status[to.requests[k_to].ix as usize];
            if from_status != StatusCode::Ok || to_status != StatusCode::Ok {
                super::status_dot_color(from_status, theme)
            } else {
                theme.line
            }
        }
        _ => theme.line,
    };

    // Правильные позиции портов: середина правой границы строки ВЫХОДА → середина левой границы строки ЗАПРОСА
    let from_port = from_obj.and_then(|fo| {
        let r = CoreRect::new(fo.rect.x, fo.rect.y, fo.rect.w, fo.rect.h);
        entry_row_port_screen(&r, &fo.outputs, polygram_core::EntryIx(edge.from), true, cell_px, camera_offset, camera_zoom)
    });

    let to_port = to_obj.and_then(|to| {
        let r = CoreRect::new(to.rect.x, to.rect.y, to.rect.w, to.rect.h);
        entry_row_port_screen(&r, &to.requests, polygram_core::EntryIx(edge.to), false, cell_px, camera_offset, camera_zoom)
    });

    // Построить ломаную: порт источника → промежуточные точки маршрута → порт приёмника.
    let mut pts: Vec<Pos2> = Vec::new();
    if let Some(fp) = from_port {
        pts.push(fp);
    }
    // Промежуточные точки из edge.points (кванты → экран), без первых и последних (их заменили на правильные порты).
    for p in edge.points.iter().skip(1).take(edge.points.len().saturating_sub(2).max(0)) {
        pts.push(Pos2::new(
            (p.x as f32 / polygram_core::layout::Q as f32 * cell_px + camera_offset.x) * camera_zoom,
            (p.y as f32 / polygram_core::layout::Q as f32 * cell_px + camera_offset.y) * camera_zoom,
        ));
    }
    if let Some(tp) = to_port {
        pts.push(tp);
    }

    if pts.len() < 2 {
        // Фоллбэк: прямая линия от порта к порту.
        if let (Some(fp), Some(tp)) = (from_port, to_port) {
            let stroke = Stroke::new(theme.edge_width * camera_zoom, line_color);
            painter.add(Shape::line(vec![fp, tp], stroke));
        }
        return;
    }

    let stroke = Stroke::new(theme.edge_width * camera_zoom, line_color);
    painter.add(Shape::line(pts, stroke));
}

/// Позиция порта выхода (правая граница объекта), в квантах. (Оставлена для совместимости.)
pub fn out_port_point(r: &CoreRect, k: usize) -> egui::Pos2 {
    egui::Pos2::new(
        (r.x + r.w) as f32 * polygram_core::layout::Q as f32,
        (r.y + 1 + k as i32) as f32 * polygram_core::layout::Q as f32
            + polygram_core::layout::Q as f32 / 2.0,
    )
}

/// Позиция порта запроса (левая граница объекта), в квантах. (Оставлена для совместимости.)
pub fn request_port_point(r: &CoreRect, k: usize) -> egui::Pos2 {
    egui::Pos2::new(
        r.x as f32 * polygram_core::layout::Q as f32,
        (r.y + 1 + k as i32) as f32 * polygram_core::layout::Q as f32
            + polygram_core::layout::Q as f32 / 2.0,
    )
}

use crate::theme::Theme;
use egui::{Painter, Shape, Stroke, Vec2};
use polygram_core::layout::Rect as CoreRect;
use polygram_core::{DynamicState, EdgeGeom, Geometry, StatusCode};

/// Позиция порта выхода (правая граница объекта), в квантах.
pub fn out_port_point(r: &CoreRect, k: usize) -> egui::Pos2 {
    egui::Pos2::new(
        (r.x + r.w) as f32 * polygram_core::layout::Q as f32,
        (r.y + 1 + k as i32) as f32 * polygram_core::layout::Q as f32
            + polygram_core::layout::Q as f32 / 2.0,
    )
}

/// Позиция порта запроса (левая граница объекта), в квантах.
pub fn request_port_point(r: &CoreRect, k: usize) -> egui::Pos2 {
    egui::Pos2::new(
        r.x as f32 * polygram_core::layout::Q as f32,
        (r.y + 1 + k as i32) as f32 * polygram_core::layout::Q as f32
            + polygram_core::layout::Q as f32 / 2.0,
    )
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
            // Определяем цвет линии по статусу source output (или target request).
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

    // Строим ортогональную ломаную из посчитанных точек маршрута (кванты → экран).
    let mut pts: Vec<egui::Pos2> = edge
        .points
        .iter()
        .map(|p| egui::Pos2::new(p.x as f32, p.y as f32))
        .collect();
    for p in &mut pts {
        *p = egui::Pos2::new(
            p.x / polygram_core::layout::Q as f32 * cell_px,
            p.y / polygram_core::layout::Q as f32 * cell_px,
        );
    }
    for p in &mut pts {
        *p = (*p + camera_offset) * camera_zoom;
    }

    if pts.is_empty() {
        // Фоллбэк: прямая линия от порта к порту, если маршрут не посчитан.
        let (from_pos, to_pos) = match (from_obj, to_obj) {
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
                let r_from = CoreRect::new(fo.rect.x, fo.rect.y, fo.rect.w, fo.rect.h);
                let r_to = CoreRect::new(to.rect.x, to.rect.y, to.rect.w, to.rect.h);
                let p_from = out_port_point(&r_from, k_from);
                let p_to = request_port_point(&r_to, k_to);
                let to_screen = |p: egui::Pos2| {
                    let px = (p.x / polygram_core::layout::Q as f32 * cell_px + camera_offset.x)
                        * camera_zoom;
                    let py = (p.y / polygram_core::layout::Q as f32 * cell_px + camera_offset.y)
                        * camera_zoom;
                    egui::Pos2::new(px, py)
                };
                (to_screen(p_from), to_screen(p_to))
            }
            _ => (egui::Pos2::ZERO, egui::Pos2::ZERO),
        };
        let stroke = Stroke::new(theme.edge_width * camera_zoom, line_color);
        painter.add(Shape::line(vec![from_pos, to_pos], stroke));
        return;
    }

    let stroke = Stroke::new(theme.edge_width * camera_zoom, line_color);
    painter.add(Shape::line(pts, stroke));
}

//! Отрисовка линий связи: цвет по статусам, ломаная со скруглёнными углами, отсечение невидимых.

use std::collections::HashSet;

use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, Vec2};
use polygram_core::geometry::{EdgeGeom, ObjectGeom};
use polygram_core::types::{Direction, StatusCode};
use polygram_core::DynamicState;

use crate::routing::screen::route_to_screen;
use crate::routing::{EdgeLayer, Side};
use crate::theme::{status_dot_color, Theme};
use crate::Camera;

/// Запас видимости вокруг рамки пары карточек (клетки).
pub const VISIBILITY_MARGIN_CELLS: f32 = 30.0;

/// Цвет линии по статусам показателей.
/// Источник «В процессе» или «Ожидается» — цвет статуса результата; иначе цель «Ожидается» —
/// цвет статуса запроса; иначе «Обеспечено». «Избыток» линию не окрашивает.
pub fn edge_color(theme: &Theme, src: StatusCode, dst: StatusCode) -> Color32 {
    if matches!(src, StatusCode::InProgress | StatusCode::Expected) {
        status_dot_color(src, Direction::Result, theme)
    } else if dst == StatusCode::Expected {
        status_dot_color(dst, Direction::Request, theme)
    } else {
        theme.status_ok
    }
}

/// Нарисовать ломаную. При `radius > 0` углы скругляются дугой, касающейся обоих отрезков;
/// радиус каждого колена ограничен `radius` и половиной меньшего из смежных отрезков, поэтому
/// короткие ступеньки скругляются слабее, а длинные — на `radius` (по умолчанию 6 px).
/// Начало и конец не скругляются.
pub fn paint_polyline(painter: &Painter, pts: &[Pos2], color: Color32, width: f32, radius: f32) {
    if pts.len() < 2 {
        return;
    }
    let stroke = Stroke::new(width, color);
    painter.add(Shape::line(rounded_points(pts, radius), stroke));
}

/// Точки ломаной после скругления углов (8 отрезков на дугу).
/// Радиус колена между отрезками `(p0->p1)` и `(p1->p2)` равен `min(radius, step / 2)`,
/// где `step` — длина меньшего из смежных отрезков. При короткой ступеньке скругление
/// адаптируется под её размер; при достаточном размере используется `radius`.
pub fn rounded_points(pts: &[Pos2], radius: f32) -> Vec<Pos2> {
    if radius <= 0.0 || pts.len() < 3 {
        return pts.to_vec();
    }
    let mut out = vec![pts[0]];
    for i in 1..pts.len() - 1 {
        let (p0, p1, p2) = (pts[i - 1], pts[i], pts[i + 1]);
        let len_in = (p1 - p0).length();
        let len_out = (p2 - p1).length();
        let step = len_in.min(len_out);
        let r = radius.min(step / 2.0);
        if r <= 0.0 {
            out.push(p1);
            continue;
        }
        let din = (p1 - p0).normalized();
        let dout = (p2 - p1).normalized();
        let center = p1 - din * r + dout * r;
        let u = -dout * r;
        let v = din * r;
        const STEPS: usize = 8;
        for k in 0..=STEPS {
            let phi = k as f32 / STEPS as f32 * std::f32::consts::FRAC_PI_2;
            out.push(center + u * phi.cos() + v * phi.sin());
        }
    }
    out.push(*pts.last().unwrap());
    out
}

/// Нарисовать все видимые линии связи поверх карточек.
#[allow(clippy::too_many_arguments)]
pub fn paint_edges(
    painter: &Painter,
    clip: Rect,
    objects: &[ObjectGeom],
    edges: &[EdgeGeom],
    layer: &mut EdgeLayer,
    dynamic: &DynamicState,
    theme: &Theme,
    camera: &Camera,
    cell_size: f32,
    drag_visual: Option<(Vec2, &HashSet<String>)>,
) {
    layer.sync(objects, edges, cell_size);
    let zoom = camera.zoom;
    let offset = (camera.offset.x, camera.offset.y);
    let to_screen = |cx: f32, cy: f32| Pos2::new((cx * cell_size + offset.0) * zoom, (cy * cell_size + offset.1) * zoom);

    let refs: Vec<_> = layer.edges().to_vec();
    for e in refs {
        let Some(frame) = layer.pair_frame(&e) else { continue };
        let margin = VISIBILITY_MARGIN_CELLS;
        let screen_frame = Rect::from_min_max(
            to_screen(frame.x as f32 - margin, frame.y as f32 - margin),
            to_screen((frame.x + frame.w) as f32 + margin, (frame.y + frame.h) as f32 + margin),
        );
        if !screen_frame.intersects(clip) {
            continue;
        }
        let Some(route) = layer.route(&e) else { continue };
        let (Some(src_e), Some(dst_e)) = (layer.entry(e.from).copied(), layer.entry(e.to).copied()) else {
            continue;
        };

        let port_screen = |layer: &EdgeLayer, entry: &crate::routing::EntryRef, side: Side| -> Pos2 {
            let card = layer.card(entry.card);
            let mut origin = to_screen(card.rect.x as f32, card.rect.y as f32);
            if let Some((shift, ids)) = &drag_visual {
                if ids.contains(&card.id) {
                    origin += *shift;
                }
            }
            let (ox, oy) = layer.port_offset_px(entry, side);
            origin + Vec2::new(ox, oy) * zoom
        };
        let src_port = port_screen(layer, &src_e, route.src_side);
        let dst_port = port_screen(layer, &dst_e, route.dst_side);
        let pts = route_to_screen(&route.points, cell_size, offset, zoom, src_port, dst_port);

        let status = |ix: u32| dynamic.status.get(ix as usize).copied().unwrap_or(StatusCode::Ok);
        let color = edge_color(theme, status(e.from), status(e.to));
        paint_polyline(painter, &pts, color, theme.edge_width * zoom, theme.line_corner_radius * zoom);
        let dot = theme.port_radius * zoom;
        if let (Some(a), Some(b)) = (pts.first(), pts.last()) {
            painter.circle_filled(*a, dot, color);
            painter.circle_filled(*b, dot, color);
        }
    }
}

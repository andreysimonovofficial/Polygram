//! Рисование одной линии связи.
//!
//! Маршрут строит и кэширует `routing::EdgeLayer` (по клеткам сетки, без экрана). Здесь —
//! только цвет по статусу и перевод маршрута в пиксели с привязкой концов к реальным точкам
//! сцепления строк показателей на текущем зуме.

use egui::{Painter, Pos2, Rect as ScreenRect, Shape, Stroke, Vec2};
use polygram_core::types::Direction;
use polygram_core::{DynamicState, EdgeGeom, EntryIx, Geometry, StatusCode};

use crate::routing::ports::visual_port_screen;
use crate::routing::screen::{cell_size_from_px, to_screen};
use crate::routing::{EdgeLayer, Route, Side};
use crate::status_dot_color;

/// Запас вокруг пары карточек (клеток), в пределах которого линия считается видимой:
/// обходы чужих карточек могут выходить за габариты пары.
const CULL_MARGIN_CELLS: f32 = 30.0;

pub fn paint_edge(
    painter: &Painter,
    edge: &EdgeGeom,
    layer: &mut EdgeLayer,
    geometry: &Geometry,
    dynamic: &DynamicState,
    theme: &crate::theme::Theme,
    cell_px: f32,
    camera_offset: Vec2,
    camera_zoom: f32,
) {
    let cell_size = cell_size_from_px(cell_px, camera_zoom);
    let Some(ends) = layer.ends(geometry, edge) else {
        return;
    };

    // Не считаем и не рисуем то, что за пределами экрана.
    let (a, b) = (ends.from.rect, ends.to.rect);
    let (x0, y0) = (a.x.min(b.x) as f32, a.y.min(b.y) as f32);
    let (x1, y1) = (
        (a.x + a.w).max(b.x + b.w) as f32,
        (a.y + a.h).max(b.y + b.h) as f32,
    );
    let m = CULL_MARGIN_CELLS;
    let area = ScreenRect::from_two_pos(
        Pos2::new(
            ((x0 - m) * cell_size + camera_offset.x) * camera_zoom,
            ((y0 - m) * cell_size + camera_offset.y) * camera_zoom,
        ),
        Pos2::new(
            ((x1 + m) * cell_size + camera_offset.x) * camera_zoom,
            ((y1 + m) * cell_size + camera_offset.y) * camera_zoom,
        ),
    );
    if !area.intersects(painter.clip_rect()) {
        return;
    }

    let from_status = dynamic.status[ends.from.outputs[ends.from_row].ix as usize];
    let to_status = dynamic.status[ends.to.requests[ends.to_row].ix as usize];
    let color = line_color(from_status, to_status, theme);

    let (from_obj, to_obj) = (ends.from, ends.to);
    let Some(route) = layer.route(geometry, edge) else {
        return;
    };

    // Концы линии — точки сцепления выбранных сторон на текущем зуме.
    let src_port = visual_port_screen(
        &from_obj.rect,
        &from_obj.outputs,
        EntryIx(edge.from),
        true,
        route.src == Side::Right,
        cell_px,
        camera_offset,
        camera_zoom,
    );
    let dst_port = visual_port_screen(
        &to_obj.rect,
        &to_obj.requests,
        EntryIx(edge.to),
        false,
        route.dst == Side::Right,
        cell_px,
        camera_offset,
        camera_zoom,
    );
    let (Some(src_port), Some(dst_port)) = (src_port, dst_port) else {
        return;
    };

    let pts = screen_polyline(
        route,
        src_port,
        dst_port,
        cell_size,
        camera_offset,
        camera_zoom,
    );
    painter.add(Shape::line(
        pts,
        Stroke::new(theme.edge_width * camera_zoom, color),
    ));
}

/// Цвет линии по статусу: сначала проблема выхода источника, потом входа приёмника.
fn line_color(from: StatusCode, to: StatusCode, theme: &crate::theme::Theme) -> egui::Color32 {
    if from != StatusCode::Ok {
        status_dot_color(from, Direction::Output, theme)
    } else if to != StatusCode::Ok {
        status_dot_color(to, Direction::Request, theme)
    } else {
        theme.status_ok
    }
}

/// Маршрут в пикселях. Первый и последний отрезки горизонтальны и начинаются в реальных
/// точках сцепления: они на текущем зуме могут отличаться от расчётных на доли пикселя.
fn screen_polyline(
    route: &Route,
    src_port: Pos2,
    dst_port: Pos2,
    cell_size: f32,
    offset: Vec2,
    zoom: f32,
) -> Vec<Pos2> {
    let mut pts: Vec<Pos2> = route
        .points
        .iter()
        .map(|&p| to_screen(p, cell_size, offset, zoom))
        .collect();
    let n = pts.len();
    if n < 2 {
        return pts;
    }
    pts[0] = src_port;
    pts[n - 1] = dst_port;
    if n == 2 {
        // Прямая линия: оставляем строго горизонтальной.
        pts[1].y = src_port.y;
    } else {
        pts[1].y = src_port.y;
        pts[n - 2].y = dst_port.y;
    }
    pts
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{vec2, Context, RawInput};

    /// Настоящий кадр egui без окна: линии строго ортогональны на любом зуме, начинаются
    /// и заканчиваются в реальных точках сцепления, первый и последний отрезки горизонтальны.
    #[test]
    fn painted_lines_are_orthogonal_and_start_at_the_ports_at_any_zoom() {
        let e = crate::routing::layer_tests::engine();
        let g = e.geometry();
        let dynamic = e.dynamic_state();
        let theme = crate::theme::Theme::default();

        for zoom in [0.4_f32, 0.75, 1.0, 1.7] {
            let cell_px = 16.0 * zoom;
            let offset = Vec2::new(30.0, 12.0);
            let mut layer = EdgeLayer::default();
            let ctx = Context::default();
            let input = RawInput {
                screen_rect: Some(ScreenRect::from_min_size(Pos2::ZERO, vec2(20000.0, 8000.0))),
                ..Default::default()
            };
            let out = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    layer.sync(&g, 16.0);
                    for edge in &g.edges {
                        paint_edge(
                            ui.painter(),
                            edge,
                            &mut layer,
                            &g,
                            &dynamic,
                            &theme,
                            cell_px,
                            offset,
                            zoom,
                        );
                    }
                });
            });

            let lines: Vec<Vec<Pos2>> = out
                .shapes
                .into_iter()
                .filter_map(|s| match s.shape {
                    Shape::Path(l) => Some(l.points),
                    _ => None,
                })
                .collect();
            assert_eq!(
                lines.len(),
                g.edges.len(),
                "зум {zoom}: нарисованы все линии"
            );

            for (edge, pts) in g.edges.iter().zip(&lines) {
                let route = layer.route(&g, edge).unwrap().clone();
                let ends = layer.ends(&g, edge).unwrap();
                let want_src = visual_port_screen(
                    &ends.from.rect,
                    &ends.from.outputs,
                    EntryIx(edge.from),
                    true,
                    route.src == Side::Right,
                    cell_px,
                    offset,
                    zoom,
                )
                .unwrap();
                let want_dst = visual_port_screen(
                    &ends.to.rect,
                    &ends.to.requests,
                    EntryIx(edge.to),
                    false,
                    route.dst == Side::Right,
                    cell_px,
                    offset,
                    zoom,
                )
                .unwrap();
                let n = pts.len();
                assert!(
                    (pts[0] - want_src).length() < 1e-3,
                    "начало в порте источника"
                );
                assert!(
                    (pts[n - 1] - want_dst).length() < 1e-3,
                    "конец в порте приёмника"
                );
                assert!(
                    (pts[0].y - pts[1].y).abs() < 1e-3,
                    "первый отрезок горизонтален"
                );
                assert!(
                    (pts[n - 2].y - pts[n - 1].y).abs() < 1e-3,
                    "последний отрезок горизонтален"
                );
                for w in pts.windows(2) {
                    let dx = (w[0].x - w[1].x).abs();
                    let dy = (w[0].y - w[1].y).abs();
                    assert!(
                        dx < 1e-3 || dy < 1e-3,
                        "зум {zoom}: диагональ {:?}->{:?}",
                        w[0],
                        w[1]
                    );
                }
            }
        }
    }
}

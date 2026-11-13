//! Рисование одной ортогональной линии связи между портами показателей.
//! Использует чистыеhelpers из `routing` (порты, сетка, коллизии, упрощение).

use egui::{Painter, Pos2, Shape, Stroke, Vec2};
use polygram_core::layout::Rect as CoreRect;
use polygram_core::types::Direction;
use polygram_core::{DynamicState, EdgeGeom, Geometry, StatusCode};

use crate::routing::{
    horizontal_blocked, obj_screen_rect, simplify_points, snap_grid_x_ceil, snap_grid_x_floor,
    visual_port_screen, vertical_blocked,
};
use crate::status_dot_color;

/// cell_size из cell_px (который уже содержит зум).
fn cell_size_from_px(cell_px: f32, camera_zoom: f32) -> f32 {
    cell_px / camera_zoom.max(0.001)
}

pub fn paint_edge(
    painter: &Painter,
    edge: &EdgeGeom,
    geometry: &Geometry,
    dynamic: &DynamicState,
    theme: &crate::theme::Theme,
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
            // Цвет линии по статусу точки: источник = Выход, приёмник = Запрос.
            if from_status != StatusCode::Ok {
                status_dot_color(from_status, Direction::Output, theme)
            } else if to_status != StatusCode::Ok {
                status_dot_color(to_status, Direction::Request, theme)
            } else {
                theme.status_ok
            }
        }
        _ => theme.line,
    };

    // Визуальные позиции точек сцепления (источник и приёмник) со стороной выхода/входа.
    // Сторона выбирается ближайшая к другому объекту: выход источника — ближе к центру цели,
    // вход приёмника — ближе к центру источника.
    // Возвращает (позиция, exits_right) — true если порт на правой границе объекта.
    let cell_size = cell_size_from_px(cell_px, camera_zoom);

    let fo_center_x = from_obj.map(|fo| {
        (fo.rect.x + fo.rect.x + fo.rect.w) as f32 * cell_size / 2.0 + camera_offset.x * camera_zoom
    });
    let to_center_x = to_obj.map(|to| {
        (to.rect.x + to.rect.x + to.rect.w) as f32 * cell_size / 2.0 + camera_offset.x * camera_zoom
    });

    let from_port_side = from_obj.and_then(|fo| {
        let from_right = visual_port_screen(
            &fo.rect,
            &fo.outputs,
            polygram_core::EntryIx(edge.from),
            true,
            true,
            cell_px,
            camera_offset,
            camera_zoom,
        );
        let from_left = visual_port_screen(
            &fo.rect,
            &fo.outputs,
            polygram_core::EntryIx(edge.from),
            true,
            false,
            cell_px,
            camera_offset,
            camera_zoom,
        );
        match (from_right, from_left, to_center_x) {
            (Some(r), Some(l), Some(tcx)) => {
                if (r.x - tcx).abs() <= (l.x - tcx).abs() {
                    Some((r, true))
                } else {
                    Some((l, false))
                }
            }
            (Some(r), None, _) => Some((r, true)),
            (None, Some(l), _) => Some((l, false)),
            (Some(r), Some(_), None) => Some((r, true)),
            (None, None, _) => None,
        }
    });

    let to_port_side = to_obj.and_then(|to| {
        let to_right = visual_port_screen(
            &to.rect,
            &to.requests,
            polygram_core::EntryIx(edge.to),
            false,
            true,
            cell_px,
            camera_offset,
            camera_zoom,
        );
        let to_left = visual_port_screen(
            &to.rect,
            &to.requests,
            polygram_core::EntryIx(edge.to),
            false,
            false,
            cell_px,
            camera_offset,
            camera_zoom,
        );
        match (to_right, to_left, fo_center_x) {
            (Some(r), Some(l), Some(fcx)) => {
                if (l.x - fcx).abs() <= (r.x - fcx).abs() {
                    Some((l, true))
                } else {
                    Some((r, false))
                }
            }
            (Some(r), None, _) => Some((r, true)),
            (None, Some(l), _) => Some((l, true)),
            (Some(_), Some(l), None) => Some((l, true)),
            (None, None, _) => None,
        }
    });

    // Линия строится только во фронтенде от визуального порта источника к визуальному порту приёмника.
    // edge.points / route.rs не используются для отрисовки.

    let mut pts: Vec<Pos2> = Vec::new();
    if let (Some((fp, from_exits_right)), Some((tp, to_enters_left))) = (from_port_side, to_port_side) {
        pts.push(fp);

        let cell_px_screen = cell_size * camera_zoom;
        let gap = 2.0 * cell_px_screen; // минимум 2 клетки от тела карточки

        let src_rect = from_obj.map(|o| obj_screen_rect(o, cell_size, camera_offset, camera_zoom));
        let dst_rect = to_obj.map(|o| obj_screen_rect(o, cell_size, camera_offset, camera_zoom));

        let from_id = from_obj.map_or("", |o| o.id.as_str());
        let to_id = to_obj.map_or("", |o| o.id.as_str());

        // V1: вертикальная линия сетки вне карточки источника со стороны выхода.
        let v1_x = if from_exits_right {
            let card_right = src_rect.map_or(fp.x, |r| r.max.x);
            let target = (card_right + gap).max(fp.x + gap);
            snap_grid_x_ceil(target, camera_offset.x, camera_zoom, cell_size)
        } else {
            let card_left = src_rect.map_or(fp.x, |r| r.min.x);
            let target = (card_left - gap).min(fp.x - gap);
            snap_grid_x_floor(target, camera_offset.x, camera_zoom, cell_size)
        };

        // V2: вертикальная линия сетки вне карточки приёмника со стороны входа.
        let v2_x = if to_enters_left {
            let card_left = dst_rect.map_or(tp.x, |r| r.min.x);
            let target = (card_left - gap).min(tp.x - gap);
            snap_grid_x_floor(target, camera_offset.x, camera_zoom, cell_size)
        } else {
            let card_right = dst_rect.map_or(tp.x, |r| r.max.x);
            let target = (card_right + gap).max(tp.x + gap);
            snap_grid_x_ceil(target, camera_offset.x, camera_zoom, cell_size)
        };

        let same_vertical = (v1_x - v2_x).abs() < 1.0;
        let vertical_free = same_vertical
            && !vertical_blocked(v1_x, fp.y, tp.y, geometry, from_id, to_id, cell_size, camera_offset, camera_zoom);

        if same_vertical && vertical_free {
            // H-V-H: fp → (V1, fp.y) → (V1, tp.y) → tp
            if (fp.x - v1_x).abs() > 1.0 {
                pts.push(Pos2::new(v1_x, fp.y));
            }
            if (fp.y - tp.y).abs() > 1.0 {
                pts.push(Pos2::new(v1_x, tp.y));
            }
            if (v1_x - tp.x).abs() > 1.0 {
                pts.push(Pos2::new(tp.x, tp.y));
            }
        } else {
            // H-V-H-V-H: разные вертикали или путь по одной вертикали заблокирован.
            let mut v2_x = v2_x;
            if same_vertical && !vertical_free {
                if from_exits_right {
                    v2_x = v1_x + cell_px_screen;
                } else {
                    v2_x = v1_x - cell_px_screen;
                }
            }

            let h_at_fp_free = !horizontal_blocked(fp.y, v1_x, v2_x, geometry, from_id, to_id, cell_size, camera_offset, camera_zoom);
            let v_at_v2_free = !vertical_blocked(v2_x, fp.y, tp.y, geometry, from_id, to_id, cell_size, camera_offset, camera_zoom);
            let v_at_v1_free = !vertical_blocked(v1_x, fp.y, tp.y, geometry, from_id, to_id, cell_size, camera_offset, camera_zoom);
            let h_at_tp_free = !horizontal_blocked(tp.y, v1_x, v2_x, geometry, from_id, to_id, cell_size, camera_offset, camera_zoom);

            if h_at_fp_free && v_at_v2_free {
                // Вариант А: горизонталь на уровне fp.y от V1 к V2, затем вертикаль по V2 вниз к tp.y.
                if (fp.x - v1_x).abs() > 1.0 {
                    pts.push(Pos2::new(v1_x, fp.y));
                }
                if (v1_x - v2_x).abs() > 1.0 {
                    pts.push(Pos2::new(v2_x, fp.y));
                }
                if (fp.y - tp.y).abs() > 1.0 {
                    pts.push(Pos2::new(v2_x, tp.y));
                }
                if (v2_x - tp.x).abs() > 1.0 {
                    pts.push(Pos2::new(tp.x, tp.y));
                }
            } else if v_at_v1_free && h_at_tp_free {
                // Вариант Б: вертикаль по V1 от fp.y к tp.y, затем горизонталь на уровне tp.y от V1 к V2.
                if (fp.x - v1_x).abs() > 1.0 {
                    pts.push(Pos2::new(v1_x, fp.y));
                }
                if (fp.y - tp.y).abs() > 1.0 {
                    pts.push(Pos2::new(v1_x, tp.y));
                }
                if (v1_x - v2_x).abs() > 1.0 {
                    pts.push(Pos2::new(v2_x, tp.y));
                }
                if (v2_x - tp.x).abs() > 1.0 {
                    pts.push(Pos2::new(tp.x, tp.y));
                }
            } else {
                // Вариант В: обход над или под препятствиями между V1 и V2.
                let x_lo = v1_x.min(v2_x);
                let x_hi = v1_x.max(v2_x);
                let y_lo = fp.y.min(tp.y);
                let y_hi = fp.y.max(tp.y);

                let mut y_over = y_lo - gap;
                let mut y_under = y_hi + gap;
                for o in &geometry.objects {
                    if o.id == from_id || o.id == to_id {
                        continue;
                    }
                    let r = obj_screen_rect(o, cell_size, camera_offset, camera_zoom);
                    if r.max.x >= x_lo && r.min.x <= x_hi && r.max.y >= y_lo && r.min.y <= y_hi {
                        y_over = y_over.min(r.min.y - gap);
                        y_under = y_under.max(r.max.y + gap);
                    }
                }

                let over_cost = (fp.y - y_over).abs() + (tp.y - y_over).abs() + (v1_x - v2_x).abs();
                let under_cost = (fp.y - y_under).abs() + (tp.y - y_under).abs() + (v1_x - v2_x).abs();
                let y_mid = if over_cost <= under_cost { y_over } else { y_under };

                if (fp.x - v1_x).abs() > 1.0 {
                    pts.push(Pos2::new(v1_x, fp.y));
                }
                if (fp.y - y_mid).abs() > 1.0 {
                    pts.push(Pos2::new(v1_x, y_mid));
                }
                if (v1_x - v2_x).abs() > 1.0 {
                    pts.push(Pos2::new(v2_x, y_mid));
                }
                if (y_mid - tp.y).abs() > 1.0 {
                    pts.push(Pos2::new(v2_x, tp.y));
                }
                if (v2_x - tp.x).abs() > 1.0 {
                    pts.push(Pos2::new(tp.x, tp.y));
                }
            }
        }
        pts.push(tp);
    }

    let pts = simplify_points(pts);

    if pts.len() < 2 {
        // Фоллбэк: прямая линия от порта к порту.
        if let (Some((fp, _)), Some((tp, _))) = (from_port_side, to_port_side) {
            let stroke = Stroke::new(theme.edge_width * camera_zoom, line_color);
            painter.add(Shape::line(vec![fp, tp], stroke));
        }
        return;
    }

    let stroke = Stroke::new(theme.edge_width * camera_zoom, line_color);
    painter.add(Shape::line(pts, stroke));
}

/// Позиция порта выхода (правая граница объекта), в квантах. (Оставлена для совместимости.)
#[allow(dead_code)]
pub fn out_port_point(r: &CoreRect, k: usize) -> egui::Pos2 {
    egui::Pos2::new(
        (r.x + r.w) as f32 * polygram_core::layout::Q as f32,
        (r.y + 1 + k as i32) as f32 * polygram_core::layout::Q as f32
            + polygram_core::layout::Q as f32 / 2.0,
    )
}

/// Позиция порта запроса (левая граница объекта), в квантах. (Оставлена для совместимости.)
#[allow(dead_code)]
pub fn request_port_point(r: &CoreRect, k: usize) -> egui::Pos2 {
    egui::Pos2::new(
        r.x as f32 * polygram_core::layout::Q as f32,
        (r.y + 1 + k as i32) as f32 * polygram_core::layout::Q as f32
            + polygram_core::layout::Q as f32 / 2.0,
    )
}

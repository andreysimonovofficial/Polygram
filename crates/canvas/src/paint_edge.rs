use crate::theme::Theme;
use crate::paint_process::{MAX_VISIBLE_METRICS, MIN_ROW_H_PX, ROW_GAP_PX, CONNECTION_DOT_RADIUS};
use egui::{Painter, Pos2, Shape, Stroke, Vec2};
use polygram_core::layout::Rect as CoreRect;
use polygram_core::{DynamicState, EdgeGeom, Geometry, StatusCode};

/// cell_size из cell_px (который уже содержит зум).
fn cell_size_from_px(cell_px: f32, camera_zoom: f32) -> f32 {
    cell_px / camera_zoom.max(0.001)
}

/// Вычислить экранную позицию точки сцепления строки показателя.
/// Логика идентична paint_process: одинаковая высота строки, зазоры между строками.
/// Точка находится внутри тела строки: слева или справа в зависимости от `exit_right`.
/// `is_output` определяет контейнер (запрос — выше, выход — ниже), `exit_right` — сторону X.
fn visual_port_screen(
    object_rect: &CoreRect,
    entries: &[polygram_core::EntryGeom],
    target_ix: polygram_core::EntryIx,
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
/// Логика идентична paint_process: одинаковая высота строки, зазоры между строками.
pub(crate) fn metric_row_center_y(
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

    let gap_total = (MAX_VISIBLE_METRICS - 1) as f32 * ROW_GAP_PX * camera_zoom;
    let row_h = if MAX_VISIBLE_METRICS > 0 {
        (content_h - gap_total) / MAX_VISIBLE_METRICS as f32
    } else {
        0.0
    };
    let row_h = row_h.max(MIN_ROW_H_PX * camera_zoom).floor();
    let row_step = row_h + ROW_GAP_PX * camera_zoom;

    let row_y = content_top + k as f32 * row_step;
    row_y + row_h / 2.0
}

/// Чистая функция: экранная X-позиция порта строки показателя.
/// exit_right = true — порт на правой границе объекта, false — на левой.
pub(crate) fn metric_port_screen_x(
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
    let dot_r = CONNECTION_DOT_RADIUS * camera_zoom;

    if exit_right {
        let object_max_x = object_min_x + object_w_px - pad;
        object_max_x - dot_r
    } else {
        let object_min_x_adj = object_min_x + pad;
        object_min_x_adj + dot_r
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
            // Цвет линии по статусу точки: источник = Выход, приёмник = Запрос.
            if from_status != StatusCode::Ok {
                super::status_dot_color(from_status, polygram_core::types::Direction::Output, theme)
            } else if to_status != StatusCode::Ok {
                super::status_dot_color(to_status, polygram_core::types::Direction::Request, theme)
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
                    Some((l, true))  // enters from left
                } else {
                    Some((r, false)) // enters from right
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

    /// Привязать экранную координату X к следующей вертикальной линии сетки (вверх/вправо).
    fn snap_grid_x_ceil(screen_x: f32, offset: f32, zoom: f32, cell_size: f32) -> f32 {
        let world = screen_x / zoom - offset;
        let cell = (world / cell_size).ceil() * cell_size;
        (cell + offset) * zoom
    }

    /// Привязать экранную координату X к предыдущей вертикальной линии сетки (вниз/влево).
    fn snap_grid_x_floor(screen_x: f32, offset: f32, zoom: f32, cell_size: f32) -> f32 {
        let world = screen_x / zoom - offset;
        let cell = (world / cell_size).floor() * cell_size;
        (cell + offset) * zoom
    }

    /// Экранный прямоугольник карточки в пикселях.
    fn obj_screen_rect(
        o: &polygram_core::ObjectGeom,
        cell_size: f32,
        offset: Vec2,
        zoom: f32,
    ) -> egui::Rect {
        egui::Rect::from_min_size(
            Pos2::new(
                (o.rect.x as f32 * cell_size + offset.x) * zoom,
                (o.rect.y as f32 * cell_size + offset.y) * zoom,
            ),
            egui::Vec2::new(
                o.rect.w as f32 * cell_size * zoom,
                o.rect.h as f32 * cell_size * zoom,
            ),
        )
    }

    /// Проверяет, пересекает ли вертикальный отрезок в x от y0 до y1 тела карточек (кроме исключённых).
    fn vertical_blocked(
        x: f32,
        y0: f32,
        y1: f32,
        geometry: &Geometry,
        exclude_from: &str,
        exclude_to: &str,
        cell_size: f32,
        offset: Vec2,
        zoom: f32,
    ) -> bool {
        let (y_min, y_max) = (y0.min(y1), y0.max(y1));
        for o in &geometry.objects {
            if o.id == exclude_from || o.id == exclude_to {
                continue;
            }
            let r = obj_screen_rect(o, cell_size, offset, zoom);
            if x >= r.min.x && x <= r.max.x && y_max >= r.min.y && y_min <= r.max.y {
                return true;
            }
        }
        false
    }

    // Построить ортогональную ломаную:
    // 1-й сегмент — горизонталь от fp к вертикальной линии сетки V1 вне карточки источника.
    // 2-й сегмент — вертикаль по V1 от fp.y до уровня обхода (или tp.y если прямой путь свободен).
    // 3-й сегмент — горизонталь от V1 к V2 на уровне обхода (только если V1 != V2).
    // 4-й сегмент — вертикаль по V2 от уровня обхода до tp.y.
    // 5-й сегмент — горизонталь от V2 к tp.
    // Все вертикальные линии привязаны к сетке. Первый и последний сегменты строго горизонтальные.
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
            // H-V-H-V-H: обход или разные вертикали.
            // Если вертикали совпали но путь заблокирован — сдвигаем V2 на одну клетку в сторону.
            let mut v2_x = v2_x;
            if same_vertical && !vertical_free {
                if from_exits_right {
                    v2_x = v1_x + cell_px_screen;
                } else {
                    v2_x = v1_x - cell_px_screen;
                }
            }

            // Уровень обхода: над или под карточками, которые пересекают вертикальную полосу между V1 и V2.
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
                // Карточка пересекает вертикальную полосу [x_lo, x_hi] по x и диапазон [y_lo, y_hi] по y.
                if r.max.x >= x_lo && r.min.x <= x_hi && r.max.y >= y_lo && r.min.y <= y_hi {
                    y_over = y_over.min(r.min.y - gap);
                    y_under = y_under.max(r.max.y + gap);
                }
            }

            let over_cost = (fp.y - y_over).abs() + (tp.y - y_over).abs() + (v1_x - v2_x).abs();
            let under_cost = (fp.y - y_under).abs() + (tp.y - y_under).abs() + (v1_x - v2_x).abs();
            let y_mid = if over_cost <= under_cost { y_over } else { y_under };

            // fp → (v1_x, fp.y)
            if (fp.x - v1_x).abs() > 1.0 {
                pts.push(Pos2::new(v1_x, fp.y));
            }
            // (v1_x, fp.y) → (v1_x, y_mid)
            if (fp.y - y_mid).abs() > 1.0 {
                pts.push(Pos2::new(v1_x, y_mid));
            }
            // (v1_x, y_mid) → (v2_x, y_mid)
            if (v1_x - v2_x).abs() > 1.0 {
                pts.push(Pos2::new(v2_x, y_mid));
            }
            // (v2_x, y_mid) → (v2_x, tp.y)
            if (y_mid - tp.y).abs() > 1.0 {
                pts.push(Pos2::new(v2_x, tp.y));
            }
            // (v2_x, tp.y) → tp
            if (v2_x - tp.x).abs() > 1.0 {
                pts.push(Pos2::new(tp.x, tp.y));
            }
        }
        pts.push(tp);
    }

    // Убираем коллинеарные подряд идущие точки (как в route::simplify ядра),
    // чтобы на вертикальных сегментах не было лишних остановок — «отростков».
    let mut simple: Vec<Pos2> = Vec::with_capacity(pts.len());
    for p in pts {
        if simple.last() == Some(&p) {
            continue;
        }
        while simple.len() >= 2 {
            let a = simple[simple.len() - 2];
            let b = simple[simple.len() - 1];
            let collinear = (a.x == b.x && b.x == p.x) || (a.y == b.y && b.y == p.y);
            if collinear {
                simple.pop();
            } else {
                break;
            }
        }
        simple.push(p);
    }
    let pts = simple;

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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_rect() -> CoreRect {
        CoreRect {
            x: 10,
            y: 20,
            w: 30,
            h: 40,
        }
    }

    #[test]
    fn metric_port_screen_x_left_right() {
        let r = test_rect();
        let cell_px = 16.0;
        let offset = egui::Vec2::ZERO;
        let zoom = 1.0;

        let left_x = metric_port_screen_x(&r, false, cell_px, offset, zoom);
        let right_x = metric_port_screen_x(&r, true, cell_px, offset, zoom);

        // Правая граница должна быть правее левой.
        assert!(right_x > left_x, "правый порт должен быть правее левого: left={}, right={}", left_x, right_x);

        // Порт на левой границе: object_min_x + pad + dot_r
        // object_min_x = 10 * 16 * 1 = 160, pad = 8, dot_r = 4 => 172
        let expected_left = (10.0 * 16.0 + 8.0 + 4.0) * 1.0;
        assert!((left_x - expected_left).abs() < 0.01, "левый порт: expected {}, got {}", expected_left, left_x);

        // Порт на правой границе: object_min_x + object_w_px - pad - dot_r
        // object_min_x = 160, object_w_px = 30 * 16 = 480, pad = 8, dot_r = 4 => 160 + 480 - 8 - 4 = 628
        let expected_right = (10.0 * 16.0 + 30.0 * 16.0 - 8.0 - 4.0) * 1.0;
        assert!((right_x - expected_right).abs() < 0.01, "правый порт: expected {}, got {}", expected_right, right_x);
    }

    #[test]
    fn metric_port_screen_x_zoom_scales() {
        let r = test_rect();
        let offset = egui::Vec2::ZERO;

        // cell_px масштабируется с zoom: base_cell_size * zoom.
        let x_zoom1 = metric_port_screen_x(&r, true, 16.0 * 1.0, offset, 1.0);
        let x_zoom2 = metric_port_screen_x(&r, true, 16.0 * 2.0, offset, 2.0);

        // При zооме 2x экранные координаты должны удваиваться.
        assert!((x_zoom2 - x_zoom1 * 2.0).abs() < 1.0, "zoom должен масштабировать X: z1={}, z2={}", x_zoom1, x_zoom2);
    }

    #[test]
    fn metric_port_screen_x_offset_shifts() {
        let r = test_rect();
        let cell_px = 16.0;
        let zoom = 1.0;

        let x_no_offset = metric_port_screen_x(&r, true, cell_px, egui::Vec2::ZERO, zoom);
        let x_with_offset = metric_port_screen_x(&r, true, cell_px, egui::Vec2::new(100.0, 0.0), zoom);

        assert!((x_with_offset - x_no_offset - 100.0).abs() < 0.01, "смещение камеры должно прибавляться к X: no_offset={}, with_offset={}", x_no_offset, x_with_offset);
    }

    #[test]
    fn metric_row_center_y_increases_with_k() {
        let r = test_rect();
        let cell_px = 16.0;
        let offset = egui::Vec2::ZERO;
        let zoom = 1.0;

        let y0 = metric_row_center_y(&r, 0, false, cell_px, offset, zoom);
        let y1 = metric_row_center_y(&r, 1, false, cell_px, offset, zoom);
        let y2 = metric_row_center_y(&r, 2, false, cell_px, offset, zoom);

        // Строки должны идти сверху вниз: y растет с k.
        assert!(y0 < y1, "строка 0 должна быть выше строки 1: y0={}, y1={}", y0, y1);
        assert!(y1 < y2, "строка 1 должна быть выше строки 2: y1={}, y2={}", y1, y2);
    }

    #[test]
    fn metric_row_center_y_output_below_request() {
        let r = test_rect();
        let cell_px = 16.0;
        let offset = egui::Vec2::ZERO;
        let zoom = 1.0;

        // Для одного и того же k выход (is_output=true) должен быть ниже запроса.
        let y_req = metric_row_center_y(&r, 0, false, cell_px, offset, zoom);
        let y_out = metric_row_center_y(&r, 0, true, cell_px, offset, zoom);

        assert!(y_out > y_req, "выход должен быть ниже запроса: y_req={}, y_out={}", y_req, y_out);
    }

    #[test]
    fn metric_row_center_y_zoom_stable() {
        let r = test_rect();
        let offset = egui::Vec2::ZERO;

        // cell_px масштабируется с zoom.
        let y_z1_k0 = metric_row_center_y(&r, 0, false, 16.0 * 1.0, offset, 1.0);
        let y_z1_k1 = metric_row_center_y(&r, 1, false, 16.0 * 1.0, offset, 1.0);
        let step_z1 = y_z1_k1 - y_z1_k0;

        let y_z2_k0 = metric_row_center_y(&r, 0, false, 16.0 * 2.0, offset, 2.0);
        let y_z2_k1 = metric_row_center_y(&r, 1, false, 16.0 * 2.0, offset, 2.0);
        let step_z2 = y_z2_k1 - y_z2_k0;

        // Шаг между строками при зум 2x должен быть примерно в 2 раза больше.
        assert!((step_z2 - step_z1 * 2.0).abs() < 1.0, "шаг строк должен масштабироваться с zoom: step_z1={}, step_z2={}", step_z1, step_z2);
    }

    #[test]
    fn visual_port_screen_matches_components() {
        use polygram_core::EntryIx;

        let r = test_rect();
        let entries = vec![polygram_core::EntryGeom {
            ix: 0,
            name: "test".into(),
            value: polygram_core::types::MetricValue::Null,
            supplied: polygram_core::types::MetricValue::Null,
            unit: "".into(),
        }];
        let cell_px = 16.0;
        let offset = egui::Vec2::ZERO;
        let zoom = 1.0;

        let port = visual_port_screen(
            &r, &entries, EntryIx(0), false, true, cell_px, offset, zoom,
        )
        .expect("порт должен существовать");

        let expected_y = metric_row_center_y(&r, 0, false, cell_px, offset, zoom);
        let expected_x = metric_port_screen_x(&r, true, cell_px, offset, zoom);

        assert!((port.y - expected_y).abs() < 0.01, "Y не совпадает: port.y={}, expected={}", port.y, expected_y);
        assert!((port.x - expected_x).abs() < 0.01, "X не совпадает: port.x={}, expected={}", port.x, expected_x);
    }
}

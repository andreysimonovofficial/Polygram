use crate::theme::Theme;
use crate::paint_process::{MAX_VISIBLE_METRICS, MIN_ROW_H_PX, ROW_GAP_PX, CONNECTION_DOT_RADIUS};
use egui::{Painter, Pos2, Shape, Stroke, Vec2};
use polygram_core::layout::Rect as CoreRect;
use polygram_core::{DynamicState, EdgeGeom, Geometry, StatusCode};

/// cell_size без зума (константа из Canvas::new).
const CELL_SIZE: f32 = 16.0;

/// cell_size из cell_px (который уже содержит зум).
fn cell_size_from_px(cell_px: f32, camera_zoom: f32) -> f32 {
    (cell_px / camera_zoom.max(0.001)).max(CELL_SIZE)
}

/// Вычислить экранную позицию точки сцепления строки показателя.
/// Логика идентична paint_process: одинаковая высота строки, зазоры между строками.
/// Точка находится внутри тела строки: слева для ЗАПРОСА, справа для ВЫХОДА.
fn visual_port_screen(
    object_rect: &CoreRect,
    entries: &[polygram_core::EntryGeom],
    target_ix: polygram_core::EntryIx,
    is_output: bool,
    cell_px: f32,
    camera_offset: Vec2,
    camera_zoom: f32,
) -> Option<Pos2> {
    let k = entries.iter().position(|e| e.ix == target_ix.0)?;

    let cell_size = cell_size_from_px(cell_px, camera_zoom);

    // Объект в экранных пикселях (как в paint_process и lib.rs)
    let object_min_x = (object_rect.x as f32 * cell_size + camera_offset.x) * camera_zoom;
    let object_min_y = (object_rect.y as f32 * cell_size + camera_offset.y) * camera_zoom;
    let object_w_px = object_rect.w as f32 * cell_px;
    let object_h_px = object_rect.h as f32 * cell_px;

    // Те же пропорции, что в paint_process
    let header_h_px = object_h_px * 0.10;
    let pad = 8.0 * camera_zoom;
    let container_rect_top = object_min_y + header_h_px + pad;
    let container_h = object_h_px - header_h_px - 2.0 * pad;
    let gap = pad;
    let split_h = container_h - gap;
    let container_h_px = split_h * 0.5;

    let container_top = if is_output {
        // ВЫХОД — второй контейнер (ниже)
        container_rect_top + container_h_px + gap
    } else {
        // ЗАПРОС — первый контейнер (выше)
        container_rect_top
    };

    let container_header_h = container_h_px * 0.20;
    let content_top = container_top + container_header_h;
    let content_h = container_h_px - container_header_h;

    // Одинаковая высота строки, не зависящая от числа показателей
    let gap_total = (MAX_VISIBLE_METRICS - 1) as f32 * ROW_GAP_PX * camera_zoom;
    let row_h = if MAX_VISIBLE_METRICS > 0 {
        (content_h - gap_total) / MAX_VISIBLE_METRICS as f32
    } else {
        0.0
    };
    let row_h = row_h.max(MIN_ROW_H_PX * camera_zoom).floor();
    let row_step = row_h + ROW_GAP_PX * camera_zoom;

    let row_y = content_top + k as f32 * row_step;
    let row_center_y = row_y + row_h / 2.0;

    let dot_r = CONNECTION_DOT_RADIUS * camera_zoom;
    if is_output {
        let object_max_x = object_min_x + object_w_px - pad;
        Some(Pos2::new(object_max_x - dot_r, row_center_y))
    } else {
        let object_min_x_adj = object_min_x + pad;
        Some(Pos2::new(object_min_x_adj + dot_r, row_center_y))
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
                theme.line
            }
        }
        _ => theme.line,
    };

    // Визуальные позиции точек сцепления (источник и приёмник).
    // Считаются той же пропорциональной логикой, что и отрисовка строк в paint_process,
    // поэтому концы линии точно ложатся в центры точек сцепления.
    let from_port = from_obj.and_then(|fo| {
        visual_port_screen(
            &fo.rect,
            &fo.outputs,
            polygram_core::EntryIx(edge.from),
            true,
            cell_px,
            camera_offset,
            camera_zoom,
        )
    });

    let to_port = to_obj.and_then(|to| {
        visual_port_screen(
            &to.rect,
            &to.requests,
            polygram_core::EntryIx(edge.to),
            false,
            cell_px,
            camera_offset,
            camera_zoom,
        )
    });

    // Промежуточные точки маршрута из edge.points (кванты → экран).
    // Исправлен подсчёт зума: cell_px уже содержит camera_zoom, поэтому
    // используется cell_size (без зума), а зум применяется один раз в конце.
    let cell_size = cell_size_from_px(cell_px, camera_zoom);
    let mut route_pts: Vec<Pos2> = Vec::new();
    for p in &edge.points {
        route_pts.push(Pos2::new(
            (p.x as f32 / polygram_core::layout::Q as f32 * cell_size + camera_offset.x) * camera_zoom,
            (p.y as f32 / polygram_core::layout::Q as f32 * cell_size + camera_offset.y) * camera_zoom,
        ));
    }

    /// Привязать экранную координату к ближайшей линии сетки.
    fn snap_to_grid(screen: f32, offset: f32, zoom: f32, cell_size: f32) -> f32 {
        let world = screen / zoom - offset;
        let cell = (world / cell_size).round() * cell_size;
        (cell + offset) * zoom
    }

    // Построить ортогональную ломаную:
    // 1-й сегмент от порта источника — горизонталь на 1 клетку за границу объекта,
    // x привязан к вертикальной линии сетки.
    // Далее — ступеньками (вертикаль → горизонталь) через промежуточные точки и цель,
    // углы привязаны к узлам сетки.
    // Последний сегмент к порту приёмника — горизонталь на 1 клетку за границу цели,
    // x привязан к вертикальной линии сетки.
    // Все сегменты строго горизонтальные или вертикальные, без диагоналей.
    let mut pts: Vec<Pos2> = Vec::new();
    if let (Some(fp), Some(tp)) = (from_port, to_port) {
        pts.push(fp);

        // Порт источника — всегда Выход (правая граница объекта): первый сегмент вправо.
        let from_elbow_x = snap_to_grid(fp.x + cell_px, camera_offset.x, camera_zoom, cell_size);
        pts.push(Pos2::new(from_elbow_x, fp.y));

        // Ступеньки через промежуточные точки и до цели.
        let mut cur = Pos2::new(from_elbow_x, fp.y);
        let mut targets: Vec<Pos2> = route_pts.clone();
        targets.push(tp);

        for wp in targets {
            // Вертикальный сегмент: текущий x (уже на линии сетки), y привязан к горизонтальной сетке.
            let wp_y = snap_to_grid(wp.y, camera_offset.y, camera_zoom, cell_size);
            if (cur.y - wp_y).abs() > 1.0 {
                pts.push(Pos2::new(cur.x, wp_y));
            }
            // Горизонтальный сегмент: y цели (уже на горизонтальной линии сетки),
            // x привязан к вертикальной линии сетки.
            let wp_x = snap_to_grid(wp.x, camera_offset.x, camera_zoom, cell_size);
            if (cur.x - wp_x).abs() > 1.0 {
                pts.push(Pos2::new(wp_x, wp_y));
            }
            cur = Pos2::new(wp_x, wp_y);
        }

        // Последний сегмент: порт приёмника — всегда Запрос (левая граница объекта),
        // локоть на 1 клетку левее порта, затем горизонтально вправо к порту.
        let tp_y_snapped = snap_to_grid(tp.y, camera_offset.y, camera_zoom, cell_size);
        if (cur.y - tp_y_snapped).abs() > 1.0 {
            pts.push(Pos2::new(cur.x, tp_y_snapped));
        }
        let to_elbow_x = snap_to_grid(tp.x - cell_px, camera_offset.x, camera_zoom, cell_size);
        if (cur.x - to_elbow_x).abs() > 1.0 {
            pts.push(Pos2::new(to_elbow_x, tp_y_snapped));
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

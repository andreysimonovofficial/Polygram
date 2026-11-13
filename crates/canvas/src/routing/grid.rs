//! Привязка к экранной сетке и проверка коллизий с телами карточек.
//! Чистая геометрия, без egui-отрисовки.

use egui::{Pos2, Vec2};
use polygram_core::geometry::{Geometry, ObjectGeom};

/// Привязать экранную координату X к следующей вертикальной линии сетки (вверх/вправо).
pub fn snap_grid_x_ceil(screen_x: f32, offset: f32, zoom: f32, cell_size: f32) -> f32 {
    let world = screen_x / zoom - offset;
    let cell = (world / cell_size).ceil() * cell_size;
    (cell + offset) * zoom
}

/// Привязать экранную координату X к предыдущей вертикальной линии сетки (вниз/влево).
pub fn snap_grid_x_floor(screen_x: f32, offset: f32, zoom: f32, cell_size: f32) -> f32 {
    let world = screen_x / zoom - offset;
    let cell = (world / cell_size).floor() * cell_size;
    (cell + offset) * zoom
}

/// Экранный прямоугольник карточки в пикселях.
pub fn obj_screen_rect(
    o: &ObjectGeom,
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
pub fn vertical_blocked(
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

/// Проверяет, пересекает ли горизонтальный отрезок в y от x0 до x1 тела карточек (кроме исключённых).
pub fn horizontal_blocked(
    y: f32,
    x0: f32,
    x1: f32,
    geometry: &Geometry,
    exclude_from: &str,
    exclude_to: &str,
    cell_size: f32,
    offset: Vec2,
    zoom: f32,
) -> bool {
    let (x_min, x_max) = (x0.min(x1), x0.max(x1));
    for o in &geometry.objects {
        if o.id == exclude_from || o.id == exclude_to {
            continue;
        }
        let r = obj_screen_rect(o, cell_size, offset, zoom);
        if y >= r.min.y && y <= r.max.y && x_max >= r.min.x && x_min <= r.max.x {
            return true;
        }
    }
    false
}

/// Убираем коллинеарные подряд идущие точки, чтобы на вертикальных сегментах не было
/// лишних остановок — «отростков».
pub fn simplify_points(pts: Vec<Pos2>) -> Vec<Pos2> {
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
    simple
}

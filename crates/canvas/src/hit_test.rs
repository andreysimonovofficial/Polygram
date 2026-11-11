use egui::Pos2;
use polygram_core::layout::Rect as CoreRect;
use polygram_core::Geometry;
use polygram_core::types::NodeKind;

fn rect_to_screen(
    object: &polygram_core::ObjectGeom,
    cell_size: f32,
    camera_offset: egui::Vec2,
    camera_zoom: f32,
) -> egui::Rect {
    let r = CoreRect::new(object.rect.x, object.rect.y, object.rect.w, object.rect.h);
    egui::Rect::from_min_size(
        world_to_screen(Pos2::new(r.x as f32, r.y as f32), cell_size, camera_offset, camera_zoom),
        egui::Vec2::new(
            r.w as f32 * cell_size * camera_zoom,
            r.h as f32 * cell_size * camera_zoom,
        ),
    )
}

/// Итерация по всем объектам (сначала обычные, потом отделы) с остановкой на первом совпадении.
fn find_object_with<'a, F>(
    geometry: &'a Geometry,
    camera_offset: egui::Vec2,
    camera_zoom: f32,
    cell_size: f32,
    mut pred: F,
) -> Option<&'a str>
where
    F: FnMut(&polygram_core::ObjectGeom, egui::Rect) -> bool,
{
    for object in &geometry.objects {
        if object.kind == NodeKind::Department {
            continue;
        }
        let rect_px = rect_to_screen(object, cell_size, camera_offset, camera_zoom);
        if pred(object, rect_px) {
            return Some(&object.id);
        }
    }
    for object in &geometry.objects {
        if object.kind != NodeKind::Department {
            continue;
        }
        let rect_px = rect_to_screen(object, cell_size, camera_offset, camera_zoom);
        if pred(object, rect_px) {
            return Some(&object.id);
        }
    }
    None
}

/// Определить объект под заданной экранной позицией.
pub fn object_at<'a>(
    geometry: &'a Geometry,
    screen_pos: Pos2,
    camera_offset: egui::Vec2,
    camera_zoom: f32,
    cell_size: f32,
) -> Option<&'a str> {
    find_object_with(geometry, camera_offset, camera_zoom, cell_size, |_, rect_px| {
        rect_px.contains(screen_pos)
    })
}

/// Вернуть id самого вложенного отдела, содержащего заданную экранную позицию.
pub fn department_at<'a>(
    geometry: &'a Geometry,
    screen_pos: Pos2,
    camera_offset: egui::Vec2,
    camera_zoom: f32,
    cell_size: f32,
) -> Option<&'a str> {
    let cell_px = cell_size * camera_zoom;
    let mut best: Option<(&'a str, f32)> = None;
    for object in &geometry.objects {
        if object.kind != NodeKind::Department {
            continue;
        }
        let rect_px = rect_to_screen(object, cell_size, camera_offset, camera_zoom);
        if rect_px.contains(screen_pos) {
            let area = rect_px.area();
            if best.map_or(true, |(_, a)| area < a) {
                best = Some((&object.id, area));
            }
        }
    }
    best.map(|(id, _)| id)
}

/// Вернуть id объекта, если экранная позиция попадает в его шапку.
pub fn header_at<'a>(
    geometry: &'a Geometry,
    screen_pos: Pos2,
    camera_offset: egui::Vec2,
    camera_zoom: f32,
    cell_size: f32,
) -> Option<&'a str> {
    let cell_px = cell_size * camera_zoom;
    find_object_with(geometry, camera_offset, camera_zoom, cell_size, |object, rect_px| {
        let header_h = if object.kind == NodeKind::Department {
            cell_px * 1.2
        } else {
            cell_px
        };
        let header_rect =
            egui::Rect::from_min_size(rect_px.min, egui::Vec2::new(rect_px.width(), header_h));
        header_rect.contains(screen_pos)
    })
}

fn world_to_screen(p: Pos2, cell_size: f32, offset: egui::Vec2, zoom: f32) -> Pos2 {
    Pos2::new(
        (p.x * cell_size + offset.x) * zoom,
        (p.y * cell_size + offset.y) * zoom,
    )
}

/// Возвращает объект, чей rect содержит клетку (cx, cy), исключая exclude_id.
pub fn object_at_cell<'a>(
    geometry: &'a Geometry,
    cx: i32,
    cy: i32,
    exclude_id: &str,
) -> Option<&'a str> {
    for object in &geometry.objects {
        if object.id == exclude_id {
            continue;
        }
        if object.rect.x <= cx
            && cx < object.rect.x + object.rect.w
            && object.rect.y <= cy
            && cy < object.rect.y + object.rect.h
        {
            return Some(&object.id);
        }
    }
    None
}

/// Свободна ли клетка (cx, cy) (нет объектов, кроме exclude_id).
pub fn is_cell_free(geometry: &Geometry, cx: i32, cy: i32, exclude_id: &str) -> bool {
    object_at_cell(geometry, cx, cy, exclude_id).is_none()
}

/// Вернута ли клетка внутри отдела dept_id.
fn inside_dept(geometry: &Geometry, dept_id: &str, cx: i32, cy: i32) -> bool {
    geometry.objects.iter().find(|o| o.id == dept_id).map_or(false, |d| {
        d.rect.x <= cx && cx < d.rect.x + d.rect.w && d.rect.y <= cy && cy < d.rect.y + d.rect.h
    })
}

/// Ближайшая свободная клетка рядом со start (спиралью по кольцам Manhattan).
/// exclude_id — объект, которого игнорируют при проверке занятости.
/// dept_id — если Some, ищет только внутри отдела dept_id.
pub fn find_free_cell_near(
    geometry: &Geometry,
    start: (i32, i32),
    max_radius: i32,
    exclude_id: &str,
    dept_id: Option<&str>,
) -> Option<(i32, i32)> {
    if is_cell_free(geometry, start.0, start.1, exclude_id)
        && dept_id.map_or(true, |d| inside_dept(geometry, d, start.0, start.1))
    {
        return Some(start);
    }
    for r in 1..=max_radius {
        for dx in -r..=r {
            for dy in -r..=r {
                if dx.abs() != r && dy.abs() != r {
                    continue;
                }
                let cx = start.0 + dx;
                let cy = start.1 + dy;
                if is_cell_free(geometry, cx, cy, exclude_id)
                    && dept_id.map_or(true, |d| inside_dept(geometry, d, cx, cy))
                {
                    return Some((cx, cy));
                }
            }
        }
    }
    None
}

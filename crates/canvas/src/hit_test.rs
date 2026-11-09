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

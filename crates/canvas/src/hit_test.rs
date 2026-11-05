use egui::Pos2;
use polygram_core::Geometry;
use polygram_core::layout::Rect as CoreRect;

/// Определить объект под заданной экранной позицией.
pub fn object_at<'a>(geometry: &'a Geometry, screen_pos: Pos2, camera_offset: egui::Vec2, camera_zoom: f32, cell_size: f32) -> Option<&'a str> {
    let cell_px = cell_size * camera_zoom;
    for object in &geometry.objects {
        if object.kind == polygram_core::types::NodeKind::Department {
            continue;
        }
        let r = CoreRect::new(object.rect.x, object.rect.y, object.rect.w, object.rect.h);
        let rect_px = egui::Rect::from_min_size(
            world_to_screen(Pos2::new(r.x as f32, r.y as f32), cell_px, camera_offset, camera_zoom),
            egui::Vec2::new(r.w as f32 * cell_px, r.h as f32 * cell_px),
        );
        if rect_px.contains(screen_pos) {
            return Some(&object.id);
        }
    }
    for object in &geometry.objects {
        if object.kind != polygram_core::types::NodeKind::Department {
            continue;
        }
        let r = CoreRect::new(object.rect.x, object.rect.y, object.rect.w, object.rect.h);
        let rect_px = egui::Rect::from_min_size(
            world_to_screen(Pos2::new(r.x as f32, r.y as f32), cell_px, camera_offset, camera_zoom),
            egui::Vec2::new(r.w as f32 * cell_px, r.h as f32 * cell_px),
        );
        if rect_px.contains(screen_pos) {
            return Some(&object.id);
        }
    }
    None
}

/// Вернуть id объекта, если экранная позиция попадает в его шапку.
pub fn header_at<'a>(geometry: &'a Geometry, screen_pos: Pos2, camera_offset: egui::Vec2, camera_zoom: f32, cell_size: f32) -> Option<&'a str> {
    let cell_px = cell_size * camera_zoom;
    for object in &geometry.objects {
        if object.kind == polygram_core::types::NodeKind::Department {
            continue;
        }
        let r = CoreRect::new(object.rect.x, object.rect.y, object.rect.w, object.rect.h);
        let rect_px = egui::Rect::from_min_size(
            world_to_screen(Pos2::new(r.x as f32, r.y as f32), cell_px, camera_offset, camera_zoom),
            egui::Vec2::new(r.w as f32 * cell_px, r.h as f32 * cell_px),
        );
        let header_h = cell_px;
        let header_rect =
            egui::Rect::from_min_size(rect_px.min, egui::Vec2::new(rect_px.width(), header_h));
        if header_rect.contains(screen_pos) {
            return Some(&object.id);
        }
    }
    for object in &geometry.objects {
        if object.kind != polygram_core::types::NodeKind::Department {
            continue;
        }
        let r = CoreRect::new(object.rect.x, object.rect.y, object.rect.w, object.rect.h);
        let rect_px = egui::Rect::from_min_size(
            world_to_screen(Pos2::new(r.x as f32, r.y as f32), cell_px, camera_offset, camera_zoom),
            egui::Vec2::new(r.w as f32 * cell_px, r.h as f32 * cell_px),
        );
        let header_h = cell_px * 1.2;
        let header_rect =
            egui::Rect::from_min_size(rect_px.min, egui::Vec2::new(rect_px.width(), header_h));
        if header_rect.contains(screen_pos) {
            return Some(&object.id);
        }
    }
    None
}

fn world_to_screen(p: Pos2, cell_size: f32, offset: egui::Vec2, zoom: f32) -> Pos2 {
    Pos2::new(
        (p.x * cell_size + offset.x) * zoom,
        (p.y * cell_size + offset.y) * zoom,
    )
}

use egui::{Pos2, Rect};
use polygram_canvas::Camera;
use polygram_core::{Geometry};
use polygram_core::types::NodeKind;

/// Обработка камеры холста: зум колёсиком к курсору и панорамирование
/// правой/средней кнопкой мыши.
pub fn handle_camera(
    ctx: &egui::Context,
    canvas_response: &egui::Response,
    camera: &mut Camera,
    cell_size: f32,
    ui: &egui::Ui,
) {
    if !canvas_response.hovered() {
        return;
    }
    // Зум колёсиком мыши к курсору (как в инструменте дизайна).
    let scroll = ctx.input(|i| i.smooth_scroll_delta());
    if scroll.y.abs() > 0.001 {
        if let Some(cursor) = ctx.pointer_hover_pos() {
            let old_zoom = camera.zoom;
            let world_cursor = Pos2::new(
                (cursor.x / old_zoom - camera.offset.x) / cell_size,
                (cursor.y / old_zoom - camera.offset.y) / cell_size,
            );
            let factor = (scroll.y * 0.01).exp();
            let new_zoom = (old_zoom * factor).clamp(0.1, 10.0);
            if new_zoom != old_zoom {
                // Новое смещение, чтобы та же мировая точка осталась под курсором.
                camera.offset.x = cursor.x / new_zoom - world_cursor.x * cell_size;
                camera.offset.y = cursor.y / new_zoom - world_cursor.y * cell_size;
                camera.zoom = new_zoom;
                ctx.request_repaint();
            }
        }
    }
    // Панорамирование правой или средней кнопкой.
    if canvas_response.dragged_by(egui::PointerButton::Secondary)
        || canvas_response.dragged_by(egui::PointerButton::Middle)
    {
        camera.offset += ui.input(|i| i.pointer.delta());
        camera.offset.x = (camera.offset.x / cell_size).round() * cell_size;
        camera.offset.y = (camera.offset.y / cell_size).round() * cell_size;
        ctx.request_repaint();
    }
}

/// Подобрать зум и смещение так, чтобы все объекты geometry вписались в viewport
/// с небольшим отступом по краям.
pub fn fit_camera_to_objects(
    camera: &mut Camera,
    geometry: &Geometry,
    viewport: Rect,
    cell_size: f32,
) {
    if geometry.objects.is_empty() {
        return;
    }
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;
    for object in &geometry.objects {
        // Процесс всегда 16×16 клеток независимо от хранимого rect.
        let (w, h) = if object.kind == NodeKind::Process { (16, 16) } else { (object.rect.w, object.rect.h) };
        min_x = min_x.min(object.rect.x);
        min_y = min_y.min(object.rect.y);
        max_x = max_x.max(object.rect.x + w);
        max_y = max_y.max(object.rect.y + h);
    }
    let min_x = min_x as f32;
    let min_y = min_y as f32;
    let max_x = max_x as f32;
    let max_y = max_y as f32;
    let box_w = (max_x - min_x).max(1.0);
    let box_h = (max_y - min_y).max(1.0);
    let center_x = (min_x + max_x) / 2.0;
    let center_y = (min_y + max_y) / 2.0;

    const PADDING: f32 = 0.06;
    let available_w = viewport.width();
    let available_h = viewport.height();
    let zoom_w = available_w / (box_w * cell_size * (1.0 + 2.0 * PADDING));
    let zoom_h = available_h / (box_h * cell_size * (1.0 + 2.0 * PADDING));
    let mut zoom = zoom_w.min(zoom_h).clamp(0.1, 10.0);

    camera.zoom = zoom;
    camera.offset.x = viewport.center().x / zoom - center_x * cell_size;
    camera.offset.y = viewport.center().y / zoom - center_y * cell_size;
}

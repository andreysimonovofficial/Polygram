use egui::{Pos2, PointerButton, Response};

/// Обработка камеры холста: зум колёсиком к курсору и панорамирование
/// правой/средней кнопкой мыши.
pub fn handle_camera(
    ctx: &egui::Context,
    canvas_response: &Response,
    camera: &mut polygram_canvas::Camera,
    cell_size: f32,
    ui: &egui::Ui,
) {
    if !canvas_response.hovered() {
        return;
    }
    // Зум колёсиком мыши к курсору (как в инструменте дизайна).
    let scroll = ctx.input(|i| i.raw_scroll_delta);
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
    if canvas_response.dragged_by(PointerButton::Secondary)
        || canvas_response.dragged_by(PointerButton::Middle)
    {
        camera.offset += ui.input(|i| i.pointer.delta());
        camera.offset.x = (camera.offset.x / cell_size).round() * cell_size;
        camera.offset.y = (camera.offset.y / cell_size).round() * cell_size;
        ctx.request_repaint();
    }
}

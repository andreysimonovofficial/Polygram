use polygram_canvas::object_at;
use polygram_core::geometry::Geometry;

/// Обработка клика по холсту: переключение выделения объекта (Ctrl+клик — добавить/убрать,
/// обычный клик — выбрать только его, клик по пустому месту — снять выделение).
pub fn handle_click_selection(
    canvas_response: &egui::Response,
    app: &mut crate::app::PolygramApp,
    geometry: &Geometry,
    ctx: &egui::Context,
) {
    if !canvas_response.clicked()
        || app.drag.drag_object().is_some()
        || app.drag.drag_metric().is_some()
        || app.drag.selection_box_start().is_some()
    {
        return;
    }
    let Some(pos) = canvas_response.interact_pointer_pos() else { return };
    let object_id = object_at(
        geometry,
        pos,
        app.canvas.camera.offset,
        app.canvas.camera.zoom,
        app.canvas.cell_size,
    );
    let ctrl = ctx.input(|i| i.modifiers.ctrl);
    match object_id {
        Some(id) => {
            if ctrl {
                if app.selected_objects.iter().any(|s| s == id) {
                    app.selected_objects.retain(|s| s != id);
                } else {
                    app.selected_objects.push(id.to_owned());
                }
            } else {
                app.selected_objects = vec![id.to_owned()];
            }
        }
        None => {
            if !ctrl {
                app.selected_objects.clear();
            }
        }
    }
}

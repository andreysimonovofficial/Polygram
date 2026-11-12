use crate::app::drag::DragInteractionState;
use crate::app::PolygramApp;
use polygram_canvas::{
    MetricBinding, paint_dashed_rect, paint_drag_preview,
};
use polygram_core::types::{Direction, MAX_UI_METRICS_PER_SIDE, MetricKind, NodeKind};
use polygram_core::{Published};
use egui::{Color32, Context, Pos2, Rect, Stroke, Vec2};
use std::collections::HashMap;

/// Основное взаимодействие с холстом: зум, панорамирование, выделение,
/// перетаскивание объектов и показателей, selection box, контекстное меню
/// по правой кнопке, отрисовка и диалоги показателей.
pub fn handle_canvas_interaction(
    app: &mut PolygramApp,
    ctx: &Context,
    ui: &mut egui::Ui,
    published: &Published,
    _close_menu: &mut bool,
) {
    // Установка области взаимодействия холста
    let canvas_response = ui.interact(
        ui.clip_rect(),
        ui.id().with("canvas"),
        egui::Sense::click_and_drag(),
    );

    // Геометрия с учётом неотправленных на сервер предсказаний позиций (drag).
    // Используется для hit-testing (object_at), чтобы клик по визуальной позиции
    // объекта попадал в тот же объект, даже если позиция ещё не подтверждена сервером.
    let geometry = app.geometry_with_predictions(published);

    // Двойной клик по объекту — инлайн-редактирование названия прямо на холсте.
    if canvas_response.hovered() && ctx.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary)) {
        if let Some(pos) = canvas_response.interact_pointer_pos() {
            if let Some(object_id) = app.canvas.object_at(&geometry, pos) {
                let obj_name = geometry
                    .objects
                    .iter()
                    .find(|o| o.id == object_id)
                    .map(|o| o.name.clone())
                    .unwrap_or_else(|| object_id.to_string());
                app.drag.state = DragInteractionState::Idle;
                app.dialogs.edit_target = Some(object_id.to_owned());
                app.dialogs.edit_input = obj_name;
            }
        }
    }

    if canvas_response.hovered() {
        super::camera::handle_camera(ctx, &canvas_response, &mut app.canvas.camera, app.canvas.cell_size, ui);

        super::selection::handle_click_selection(&canvas_response, app, &geometry, ctx);
        super::drag_object::handle_drag(app, ctx, ui, &canvas_response, &geometry);

        // Контекстное меню по правой кнопке (только клик, не перетаскивание).
        if canvas_response.secondary_clicked() {
            if let Some(pos) = canvas_response.interact_pointer_pos() {
                let object_id = app.canvas.object_at(&geometry, pos);
                app.right_click = Some((pos, object_id.map(|s| s.to_owned())));
            }
        }
    }

    // Ховер над объектом для подсветки рамки и текста.
    let hovered_object = if canvas_response.hovered() {
        if let Some(pos) = canvas_response.interact_pointer_pos() {
            app.canvas.object_at(&geometry, pos)
        } else {
            None
        }
    } else {
        None
    };

    // Собрать привязки по ключу (object_id, entry_name) для отрисовки иконки + времени
    // последнего опроса внутри строк показателя (масштабируются с zoom).
    let mut bindings: HashMap<(String, String), MetricBinding> = HashMap::new();
    for (id, info) in &app.bindings.api_bindings {
        if let Some((object_id, entry_name)) = id.split_once(':') {
            bindings.insert(
                (object_id.to_owned(), entry_name.to_owned()),
                MetricBinding {
                    ok: info.ok,
                    last_updated: info.last_updated.clone(),
                    error: info.error.clone(),
                },
            );
        }
    }

    app.canvas.paint(ui, &app.geometry_with_predictions(&published), &published.dynamic, hovered_object, app.dialogs.edit_target.as_deref(), &app.selected_objects, app.show_grid, &bindings);

    // Перетаскивание показателя: графическая копия строки у курсора и подсветка целевого объекта (рисуется поверх холста).
    if app.drag.drag_metric().is_some()
        && canvas_response.dragged_by(egui::PointerButton::Primary)
    {
        if let Some(cur) = canvas_response.interact_pointer_pos() {
            if let Some((source_object, source_dir, entry)) = &app.drag.drag_metric() {
                let painter = ui.painter();
                let preview_w = 220.0 * app.canvas.camera.zoom;
                let preview_h = 27.0 * app.canvas.camera.zoom;
                let preview_rect = Rect::from_min_size(
                    cur + Vec2::new(14.0, 14.0),
                    Vec2::new(preview_w, preview_h),
                );
                let binding = bindings.get(&(source_object.clone(), entry.name.clone()));
                paint_drag_preview(
                    painter,
                    preview_rect,
                    entry,
                    &published.dynamic,
                    app.canvas.camera.zoom,
                    &app.canvas.theme,
                    *source_dir,
                    binding,
                );
                if let Some(target_id) = app
                    .canvas
                    .object_at(&geometry, cur)
                    .filter(|id| *id != *source_object)
                {
                    if let Some(target) = published
                        .geometry
                        .objects
                        .iter()
                        .find(|o| o.id == target_id)
                    {
                        if target.kind == NodeKind::Process {
                            let r = polygram_core::layout::Rect::new(
                                target.rect.x,
                                target.rect.y,
                                target.rect.w,
                                target.rect.h,
                            );
                            let rect_px = Rect::from_min_size(
                                app.canvas.camera.world_to_screen(
                                    Pos2::new(r.x as f32, r.y as f32),
                                    app.canvas.cell_size,
                                ),
                                Vec2::new(
                                    r.w as f32 * app.canvas.cell_px(),
                                    r.h as f32 * app.canvas.cell_px(),
                                ),
                            );
                            painter.rect_stroke(
                                rect_px.expand(2.0),
                                2.0,
                                Stroke::new(
                                    2.0_f32 * app.canvas.camera.zoom,
                                    Color32::from_rgb(0, 217, 255),
                                ),
                            );
                        }
                    }
                }
            }
        }
    }

    // Перетаскивание объекта: подсветить отдел под курсором (как процесс подсвечивается,
    // когда в него перетаскивают показатель).
    if app.drag.drag_object().is_some()
        && canvas_response.dragged_by(egui::PointerButton::Primary)
    {
        if let Some(cur) = canvas_response.interact_pointer_pos() {
            let dragged_id = app.drag.drag_object().map(|s| s.to_string()).unwrap_or_default();
            let cell_px = app.canvas.cell_px();
            let offset = app.canvas.camera.offset;
            let zoom = app.canvas.camera.zoom;
            if let Some((_dept_id, rect_px)) = geometry.objects.iter().find_map(|o| {
                if o.kind != NodeKind::Department || o.id == dragged_id {
                    return None;
                }
                let r = polygram_core::layout::Rect::new(o.rect.x, o.rect.y, o.rect.w, o.rect.h);
                let rect_px = Rect::from_min_size(
                    Pos2::new(r.x as f32, r.y as f32) * cell_px + offset * zoom,
                    Vec2::new(r.w as f32 * cell_px, r.h as f32 * cell_px),
                );
                rect_px.contains(cur).then_some((o.id.clone(), rect_px))
            }) {
                let painter = ui.painter();
                painter.rect_stroke(
                    rect_px.expand(2.0),
                    2.0,
                    Stroke::new(2.0_f32 * zoom, Color32::WHITE),
                );
            }
        }
    }

    // Перетаскивание объекта: нарисовать пустую пунктирную рамку ("призрачная копия")
    // в клетке, куда объект уедет при Drop, если цель занята другим объектом или отделом.
    if app.drag.drag_ghost.is_some()
        && canvas_response.dragged_by(egui::PointerButton::Primary)
    {
        if let Some((gx, gy)) = app.drag.drag_ghost {
            if let Some(dragged) = geometry.objects.iter().find(|o| {
                o.id == app.drag.drag_object().as_deref().unwrap_or("")
            }) {
                let cell_px = app.canvas.cell_px();
                let offset = app.canvas.camera.offset;
                let zoom = app.canvas.camera.zoom;
                let rect_px = Rect::from_min_size(
                    Pos2::new(gx as f32, gy as f32) * cell_px + offset * zoom,
                    Vec2::new(
                        dragged.rect.w as f32 * cell_px,
                        dragged.rect.h as f32 * cell_px,
                    ),
                );
                let painter = ui.painter();
                paint_dashed_rect(painter, rect_px, zoom, Color32::from_rgb(0, 217, 255));
            }
        }
    }

    // Selection Box: рисуем резиновый прямоугольник поверх холста.
    if app.drag.selection_box_start().is_some()
        && canvas_response.dragged_by(egui::PointerButton::Primary)
    {
        if let (Some(start), Some(cur)) = (app.drag.selection_box_start(), canvas_response.interact_pointer_pos()) {
            let painter = ui.painter();
            let x = start.x.min(cur.x);
            let y = start.y.min(cur.y);
            let w = (cur.x - start.x).abs();
            let h = (cur.y - start.y).abs();
            if w > 1.0 && h > 1.0 {
                let box_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h));
                painter.rect_stroke(
                    box_rect,
                    0.0,
                    Stroke::new(2.0_f32, Color32::from_rgb(0, 217, 255)),
                );
            }
        }
    }

    // Копировать текст ошибки API-привязки по клику на иконку ⚠ в строке показателя.
    if canvas_response.clicked() {
        if let Some(pos) = canvas_response.interact_pointer_pos() {
            for (rect, error_text) in &app.canvas.api_error_hotspots {
                if rect.contains(pos) {
                    ctx.copy_text(error_text.clone());
                    break;
                }
            }
        }
    }

    // Добавить показатель по клику на строку "+ Добавить показатель" — открыть диалог.
    if let Some(pos) = canvas_response.interact_pointer_pos() {
        let is_double = ctx.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary));
        for (object_id, dir, rect) in &app.canvas.add_metric_hotspots {
            if rect.contains(pos) && canvas_response.clicked() && !is_double {
                let n = match dir {
                    Direction::Request => published
                        .geometry
                        .objects
                        .iter()
                        .find(|o| o.id == *object_id)
                        .map(|o| o.requests.len())
                        .unwrap_or(0),
                    Direction::Output => published
                        .geometry
                        .objects
                        .iter()
                        .find(|o| o.id == *object_id)
                        .map(|o| o.outputs.len())
                        .unwrap_or(0),
                };
                if n >= MAX_UI_METRICS_PER_SIDE {
                    continue;
                }
                app.dialogs.add_metric_target = Some((object_id.clone(), *dir));
                app.dialogs.add_metric_name = match dir {
                    Direction::Request => format!("request{}", n + 1),
                    Direction::Output => format!("out{}", n + 1),
                };
                let existing_kind = published
                    .metric_defs
                    .iter()
                    .find(|md| md.name == app.dialogs.add_metric_name)
                    .map(|md| md.kind)
                    .unwrap_or(MetricKind::Number);
                app.dialogs.add_metric_kind = existing_kind;
                app.dialogs.add_metric_value = "0".to_owned();
                app.dialogs.add_metric_date = None;
                app.dialogs.add_metric_date_popup_open = false;
                app.dialogs.add_metric_time = None;
                app.dialogs.add_metric_unit = String::new();
                app.drag.state = DragInteractionState::Idle;
                break;
            }
        }
    }

    super::add_metric::handle_add_metric_dialog(app, ctx, published);

    // Открыть диалог редактирования списка показателей контейнера по клику на ≡.
    if let Some(pos) = canvas_response.interact_pointer_pos() {
        let is_double = ctx.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary));
        for (object_id, dir, rect) in &app.canvas.container_menu_hotspots {
            if rect.contains(pos) && canvas_response.clicked() && !is_double {
                app.dialogs.container_menu_target = Some((object_id.clone(), *dir));
                app.drag.state = DragInteractionState::Idle;
                break;
            }
        }
    }

    super::container_menu::handle_container_menu_dialog(app, ctx, published);
}

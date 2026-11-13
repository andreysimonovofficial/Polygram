use crate::app::drag::DragInteractionState;
use crate::app::PolygramApp;
use polygram_canvas::object_at;
use polygram_core::drag_domain::{compute_drag_preview, DragPreviewInput};
use polygram_core::geometry::Geometry;
use polygram_core::layout::Rect as CoreRect;
use polygram_core::types::{Direction, NodeKind};
use polygram_core::{Command};
use egui::{Context, Pos2, Rect, Response, PointerButton, Vec2};

/// Вся логика перетаскивания: старт (объект / показатель / selection box),
/// клиентское предсказание позиции, призрачная копия, дроп объекта и показателя,
/// завершение выделения рамкой.
///
/// UI-слой (egui) только читает ввод и вызывает чистые доменные функции.
pub fn handle_drag(
    app: &mut PolygramApp,
    ctx: &Context,
    ui: &egui::Ui,
    canvas_response: &Response,
    geometry: &Geometry,
) {
    // ---- Старт взаимодействия ----
    if canvas_response.drag_started_by(PointerButton::Primary)
        && matches!(app.drag.state, DragInteractionState::Idle)
    {
        if let Some(pos) = canvas_response.interact_pointer_pos() {
            let metric_drag = app
                .canvas
                .drag_metric_hotspots
                .iter()
                .find(|(_, _, _, rect)| rect.contains(pos));
            if let Some((object_id, dir, metric_name, _)) = metric_drag {
                let entry = geometry.objects.iter()
                    .find(|o| o.id == *object_id)
                    .and_then(|o| match dir {
                        Direction::Request => o.requests.iter().find(|e| e.name == *metric_name),
                        Direction::Output => o.outputs.iter().find(|e| e.name == *metric_name),
                    })
                    .cloned();
                if let Some(entry) = entry {
                    let entry_name = entry.name.clone();
                    app.drag.start_drag_metric(
                        object_id.clone(),
                        *dir,
                        entry,
                        pos,
                    );
                    app.add_event_log(
                        "info",
                        "drag",
                        &format!("Начат перетаскивание показателя «{entry_name}» из объекта «{object_id}»"),
                        None,
                    );
                }
            } else if let Some(object_id) = object_at(
                geometry,
                pos,
                app.canvas.camera.offset,
                app.canvas.camera.zoom,
                app.canvas.cell_size,
            ) {
                let object = geometry
                    .objects
                    .iter()
                    .find(|o| o.id == *object_id);
                if let Some(o) = object {
                    app.drag.start_drag_object(
                        object_id.to_owned(),
                        pos,
                        (o.rect.x, o.rect.y),
                    );
                    app.add_event_log(
                        "info",
                        "drag",
                        &format!("Начат перетаскивание объекта «{object_id}» с клетки ({}, {})", o.rect.x, o.rect.y),
                        None,
                    );
                }
            } else {
                app.drag.start_selection_box(pos);
                app.add_event_log("info", "drag", "Начато выделение рамкой", None);
            }
        }
    }

    // ---- Обновление превью перетаскивания объекта ----
    if matches!(app.drag.state, DragInteractionState::DraggingObject { .. })
        && canvas_response.dragged_by(PointerButton::Primary)
    {
        if let Some(cur) = canvas_response.interact_pointer_pos() {
            let object_id = app.drag.drag_object().map(|s| s.to_string()).unwrap_or_default();
            let _start_cell = app.drag.drag_start_cell().unwrap_or((0, 0));
            let dragged_size = geometry
                .objects
                .iter()
                .find(|o| o.id == object_id)
                .map(|o| (o.rect.w, o.rect.h))
                .unwrap_or((0, 0));

            let target_x = ((cur.x / app.canvas.camera.zoom - app.canvas.camera.offset.x) / app.canvas.cell_size).round() as i32;
            let target_y = ((cur.y / app.canvas.camera.zoom - app.canvas.camera.offset.y) / app.canvas.cell_size).round() as i32;
            let cursor_cell = (target_x, target_y);

            let input = DragPreviewInput {
                geometry: geometry.clone(),
                dragged_id: object_id.to_string(),
                dragged_size,
                cursor_cell,
            };
            let preview = compute_drag_preview(&input);

            app.drag.drag_ghost = preview.ghost;
            app.drag.drag_over = preview.over;

            let can_move = true;
            if can_move {
                app.drag.update_prediction(&object_id, preview.predicted);
            }
        }
    }

    // ---- Завершение перетаскивания объекта ----
    if matches!(app.drag.state, DragInteractionState::DraggingObject { .. } | DragInteractionState::DroppingObject { .. })
        && !ui.input(|i| i.pointer.button_down(PointerButton::Primary))
    {
        // Захватить состояние наведения до очистки в end_drag_object.
        let drag_over = app.drag.drag_over.clone();
        let drag_ghost = app.drag.drag_ghost;

        app.drag.begin_drop_object(None);
        let (drag_object, drag_start_cell, _target) =
            match app.drag.end_drag_object() {
                Some(v) => v,
                None => return,
            };

        let object_id = drag_object;
        let start_x = drag_start_cell.0;
        let start_y = drag_start_cell.1;

        let predicted = app
            .drag
            .predicted_positions
            .get(&object_id)
            .copied()
            .unwrap_or((start_x, start_y));
        let dragged_size = geometry
            .objects
            .iter()
            .find(|o| o.id == object_id)
            .map(|o| (o.rect.w, o.rect.h));
        let drop_cell = drag_ghost.unwrap_or(predicted);
        let drop_in_dept = drag_over.as_deref().map_or(false, |dept_id| {
            geometry.objects.iter().any(|o| o.id == dept_id && o.kind == NodeKind::Department)
        });
        // Отдел тянется под своих детей (fit_departments_to_children), поэтому
        // его rect охватывает перетаскиваемый ребёнка и не должен считаться
        // занятым местом при перемещении внутри отдела.
        let exclude_overlap = if drop_in_dept {
            drag_over.as_deref()
        } else {
            None
        };
        let target_overlaps = dragged_size.is_some_and(|(w, h)| {
            let pr = CoreRect::new(drop_cell.0, drop_cell.1, w, h);
            geometry.objects.iter().any(|o| {
                if o.id == object_id {
                    return false;
                }
                if exclude_overlap.map_or(false, |d| o.id == d) {
                    return false;
                }
                o.rect.overlaps(&pr)
            })
        });

        if drop_in_dept {
            let dept_id = drag_over.expect("drop_in_dept requires drag_over");
            // Если процесс уже состоит в этом отделе — это перемещение
            // внутри отдела, а не добавление в отдел. Модальное окно не показываем.
            let already_in_dept = geometry.objects.iter().any(|o| {
                o.id == object_id && o.parent.as_deref() == Some(&dept_id)
            });
            if already_in_dept {
                let pos_changed = (drop_cell.0, drop_cell.1) != (start_x, start_y);
                if pos_changed && !target_overlaps {
                    app.drag.pending_move =
                        Some((object_id.clone(), app.connection.last_rev));
                    app.apply_cmd(Command::MoveObject {
                        object: object_id.clone(),
                        at: [drop_cell.0, drop_cell.1],
                    });
                    app.add_event_log(
                        "info",
                        "drag",
                        &format!(
                            "Объект «{object_id}» перемещён с ({start_x}, {start_y}) на ({}, {}) в отделе «{}»",
                            drop_cell.0, drop_cell.1, dept_id
                        ),
                        None,
                    );
                } else {
                    app.drag.predicted_positions.remove(&object_id);
                    app.add_event_log(
                        "warn",
                        "drag",
                        &format!("Перетаскивание объекта «{object_id}» отменено (позиция не изменилась или цель занята)"),
                        None,
                    );
                }
            } else {
                // Открыть модальное подтверждение переноса в отдел.
                app.dialogs.reparent_confirm = Some((
                    object_id.clone(),
                    dept_id,
                    drop_cell,
                    (start_x, start_y),
                ));
            }
        } else {
            let skip_move = drag_over.is_some() && drag_ghost.is_none();
            if !skip_move && !target_overlaps {
                let pos_changed = (drop_cell.0, drop_cell.1) != (start_x, start_y);
                if pos_changed {
                    app.drag.pending_move =
                        Some((object_id.clone(), app.connection.last_rev));
                    app.apply_cmd(Command::MoveObject {
                        object: object_id.clone(),
                        at: [drop_cell.0, drop_cell.1],
                    });
                    app.add_event_log(
                        "info",
                        "drag",
                        &format!(
                            "Объект «{object_id}» перемещён с ({start_x}, {start_y}) на ({}, {})",
                            drop_cell.0, drop_cell.1
                        ),
                        None,
                    );
                }
            } else {
                app.drag.predicted_positions.remove(&object_id);
                app.add_event_log(
                    "warn",
                    "drag",
                    &format!("Перетаскивание объекта «{object_id}» отменено (цель занята или отсутствует призрачная копия)"),
                    None,
                );
            }
        }
        app.drag.drag_ghost = None;
        app.drag.drag_over = None;
        app.drag.state = DragInteractionState::Idle;
    }

    // ---- Завершение перетаскивания показателя ----
    if matches!(app.drag.state, DragInteractionState::DraggingMetric { .. } | DragInteractionState::DroppingMetric { .. })
        && !ui.input(|i| i.pointer.button_down(PointerButton::Primary))
    {
        app.drag.begin_drop_metric();
        let (source_object, source_dir, entry) =
            match app.drag.end_drag_metric() {
                Some(v) => v,
                None => return,
            };
        let metric_name = entry.name.clone();
        if let Some(pos) = canvas_response.interact_pointer_pos() {
            if let Some(target_id) = app
                .canvas
                .object_at(&geometry, pos)
                .map(|s| s.to_owned())
            {
                let ctx = polygram_core::drag_domain::MetricDropContext {
                    geometry: geometry.clone(),
                    source_object: source_object.clone(),
                    source_dir,
                    entry_name: metric_name.clone(),
                    target_id: target_id.clone(),
                };
                let cmds = polygram_core::drag_domain::compute_metric_drop(&ctx);
                if !cmds.is_empty() {
                    app.apply_batch(cmds);
                    app.add_event_log(
                        "info",
                        "drag",
                        &format!(
                            "Показатель «{metric_name}» соединён: «{source_object}» → «{target_id}»"
                        ),
                        None,
                    );
                }
            }
        }
        app.drag.state = DragInteractionState::Idle;
    }

    // ---- Завершение выделения рамкой ----
    if matches!(app.drag.state, DragInteractionState::SelectingBox { .. })
        && !ui.input(|i| i.pointer.button_down(PointerButton::Primary))
    {
        if let (Some(start), Some(end)) = (app.drag.selection_box_start(), canvas_response.interact_pointer_pos()) {
            let x = start.x.min(end.x);
            let y = start.y.min(end.y);
            let w = (end.x - start.x).abs();
            let h = (end.y - start.y).abs();
            if w > 4.0 && h > 4.0 {
                let cell_px = app.canvas.cell_px();
                let box_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h));
                let ctrl = ctx.input(|i| i.modifiers.ctrl);
                let mut newly_selected: Vec<String> = Vec::new();
                for object in &geometry.objects {
                    let r = CoreRect::new(object.rect.x, object.rect.y, object.rect.w, object.rect.h);
                    let rect_px = Rect::from_min_size(
                        app.canvas.camera.world_to_screen(Pos2::new(r.x as f32, r.y as f32), app.canvas.cell_size),
                        Vec2::new(r.w as f32 * cell_px, r.h as f32 * cell_px),
                    );
                    if box_rect.intersects(rect_px) {
                        newly_selected.push(object.id.clone());
                    }
                }
                if ctrl {
                    for id in &newly_selected {
                        if !app.selected_objects.iter().any(|s| s == id) {
                            app.selected_objects.push(id.clone());
                        }
                    }
                } else {
                    app.selected_objects = newly_selected;
                }
            }
            app.drag.state = DragInteractionState::Idle;
        }
    }
}

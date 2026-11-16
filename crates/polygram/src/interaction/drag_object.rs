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
                let found = geometry.objects.iter()
                    .find(|o| o.id == *object_id)
                    .and_then(|o| {
                        let entries = match dir {
                            Direction::Request => &o.requests,
                            Direction::Result => &o.results,
                        };
                        entries.iter().enumerate().find(|(_, e)| e.name == *metric_name)
                            .map(|(i, e)| (e.clone(), i))
                    });
                if let Some((entry, from_ix)) = found {
                    let entry_name = entry.name.clone();
                    app.drag.start_drag_metric(
                        object_id.clone(),
                        *dir,
                        entry,
                        from_ix,
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
                    let click_world =
                        app.canvas.camera.screen_to_world(pos, app.canvas.cell_size);
                    let card_top_left_cell =
                        Pos2::new(o.rect.x as f32, o.rect.y as f32);
                    let drag_offset_cell = click_world - card_top_left_cell;
                    app.drag.start_drag_object(
                        object_id.to_owned(),
                        pos,
                        (o.rect.x, o.rect.y),
                        drag_offset_cell,
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
            let drag_offset_cell = app.drag.drag_offset_cell().unwrap_or_default();
            let cursor_world =
                app.canvas.camera.screen_to_world(cur, app.canvas.cell_size);
            let target_world = cursor_world - drag_offset_cell;
            let target_x = target_world.x.round() as i32;
            let target_y = target_world.y.round() as i32;
            let cursor_cell = (target_x, target_y);

            let dragged_size = geometry
                .objects
                .iter()
                .find(|o| o.id == object_id)
                .map(|o| {
                    if o.kind == NodeKind::Process {
                        (16, 16)
                    } else {
                        (o.rect.w, o.rect.h)
                    }
                })
                .unwrap_or((0, 0));

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
                app.drag.update_prediction(&object_id, cursor_cell);
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
        // Дети перетаскиваемого отдела едут вместе с ним, поэтому их rect
        // тоже не считается занятым местом при перемещении родителя.
        let dragged_children: std::collections::HashSet<&str> = geometry
            .objects
            .iter()
            .filter(|o| o.parent.as_deref() == Some(&object_id))
            .map(|o| o.id.as_str())
            .collect();
        let target_overlaps = dragged_size.is_some_and(|(w, h)| {
            let pr = CoreRect::new(drop_cell.0, drop_cell.1, w, h);
            geometry.objects.iter().any(|o| {
                if o.id == object_id {
                    return false;
                }
                if dragged_children.contains(o.id.as_str()) {
                    return false;
                }
                if exclude_overlap.map_or(false, |d| o.id == d) {
                    return false;
                }
                o.rect.overlaps(&pr)
            })
        });

            if drop_in_dept {
            let dept_id = drag_over.clone().expect("drop_in_dept requires drag_over");
            // Если объект уже состоит в этом отделе — это перемещение
            // внутри отдела, а не добавление в отдел. Модальное окно не показываем.
            let already_in_dept = geometry.objects.iter().any(|o| {
                o.id == object_id && o.parent.as_deref() == Some(&dept_id)
            });
            let object_kind = geometry
                .objects
                .iter()
                .find(|o| o.id == object_id)
                .map(|o| o.kind)
                .unwrap_or(NodeKind::Process);
            if already_in_dept {
                let pos_changed = (drop_cell.0, drop_cell.1) != (start_x, start_y);
                // При перемещении внутри отдела исключаем всех детей целевого отдела
                // из проверки наложения — отдел растянется под детей через
                // fit_departments_to_children, поэтому дети могут перестраиваться
                // внутри него без конфликтов.
                let target_overlaps_in_dept = dragged_size.is_some_and(|(w, h)| {
                    let pr = CoreRect::new(drop_cell.0, drop_cell.1, w, h);
                    geometry.objects.iter().any(|o| {
                        if o.id == object_id {
                            return false;
                        }
                        if dragged_children.contains(o.id.as_str()) {
                            return false;
                        }
                        if exclude_overlap.map_or(false, |d| o.id == d) {
                            return false;
                        }
                        if o.parent.as_deref() == Some(&dept_id) {
                            return false;
                        }
                        o.rect.overlaps(&pr)
                    })
                });
                if pos_changed && !target_overlaps_in_dept {
                    app.drag.pending_move =
                        Some((object_id.clone(), app.connection.last_rev));
                    let dx = drop_cell.0 - start_x;
                    let dy = drop_cell.1 - start_y;
                    let mut cmds: Vec<Command> = vec![Command::MoveObject {
                        object: object_id.clone(),
                        at: [drop_cell.0, drop_cell.1],
                    }];
                    // Отдел перевозит всех потомков с собой: без этого после
                    // подтверждения сервером fit_departments_to_children вернёт
                    // отдел обратно к детям на старых позициях ("прыжок назад").
                    if geometry.objects.iter().any(|o| o.id == object_id && o.kind == NodeKind::Department) {
                        if let Some(pub_geom) = app.connection.snapshot.as_ref().map(|s| &s.geometry) {
                            let mut queue: Vec<String> = pub_geom.objects.iter()
                                .filter(|o| o.parent.as_deref() == Some(&object_id))
                                .map(|o| o.id.clone())
                                .collect();
                            while let Some(child_id) = queue.pop() {
                                if let Some(child) = pub_geom.objects.iter().find(|o| o.id == child_id) {
                                    cmds.push(Command::MoveObject {
                                        object: child_id.clone(),
                                        at: [child.rect.x + dx, child.rect.y + dy],
                                    });
                                    if child.kind == NodeKind::Department {
                                        queue.extend(pub_geom.objects.iter()
                                            .filter(|o| o.parent.as_deref() == Some(&child_id))
                                            .map(|o| o.id.clone()));
                                    }
                                }
                            }
                        }
                    }
                    app.apply_batch(cmds);
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
                    object_kind,
                ));
            }
        } else {
            let skip_move = drag_over.is_some() && drag_ghost.is_none();
            if !skip_move && !target_overlaps {
                let pos_changed = (drop_cell.0, drop_cell.1) != (start_x, start_y);
                if pos_changed {
                    app.drag.pending_move =
                        Some((object_id.clone(), app.connection.last_rev));
                    let dx = drop_cell.0 - start_x;
                    let dy = drop_cell.1 - start_y;
                    let mut cmds: Vec<Command> = vec![Command::MoveObject {
                        object: object_id.clone(),
                        at: [drop_cell.0, drop_cell.1],
                    }];
                    if geometry.objects.iter().any(|o| o.id == object_id && o.kind == NodeKind::Department) {
                        if let Some(pub_geom) = app.connection.snapshot.as_ref().map(|s| &s.geometry) {
                            let mut queue: Vec<String> = pub_geom.objects.iter()
                                .filter(|o| o.parent.as_deref() == Some(&object_id))
                                .map(|o| o.id.clone())
                                .collect();
                            while let Some(child_id) = queue.pop() {
                                if let Some(child) = pub_geom.objects.iter().find(|o| o.id == child_id) {
                                    cmds.push(Command::MoveObject {
                                        object: child_id.clone(),
                                        at: [child.rect.x + dx, child.rect.y + dy],
                                    });
                                    if child.kind == NodeKind::Department {
                                        queue.extend(pub_geom.objects.iter()
                                            .filter(|o| o.parent.as_deref() == Some(&child_id))
                                            .map(|o| o.id.clone()));
                                    }
                                }
                            }
                        }
                    }
                    app.apply_batch(cmds);
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
        let (source_object, source_dir, entry, from_ix) =
            match app.drag.end_drag_metric() {
                Some(v) => v,
                None => return,
            };
        let metric_name = entry.name.clone();
        if let Some(pos) = canvas_response.interact_pointer_pos() {
            // Реордер строки внутри того же контейнера (ЗАПРОС/РЕЗУЛЬТАТ одного объекта).
            let hovered_name = app.canvas.drag_metric_hotspots.iter()
                .find(|(oid, d, name, rect)| {
                    *oid == source_object && *d == source_dir && rect.contains(pos)
                })
                .map(|(_, _, name, _)| name.clone());
            if let Some(hovered_name) = hovered_name {
                if let Some(obj) = geometry.objects.iter().find(|o| o.id == source_object) {
                    let entries = match source_dir {
                        Direction::Request => &obj.requests,
                        Direction::Result => &obj.results,
                    };
                    if let Some(to_ix) = entries.iter().position(|e| e.name == hovered_name) {
                        if to_ix != from_ix {
                            // Запомнить реордер как pending, чтобы применить к снимкам от сервера.
                            app.drag.pending_reorder = Some((
                                source_object.clone(),
                                source_dir,
                                from_ix,
                                to_ix,
                            ));
                            // Клиентское предсказание: сразу применяем к текущему снимку.
                            app.connection.apply_reorder_locally(
                                &source_object,
                                source_dir,
                                from_ix,
                                to_ix,
                            );
                            app.apply_cmd(Command::ReorderMetricEntry {
                                object: source_object.clone(),
                                dir: source_dir,
                                from_ix,
                                to_ix,
                            });
                            app.add_event_log(
                                "info",
                                "drag",
                                &format!(
                                    "Показатель «{metric_name}» перемещён с позиции {from_ix} на {to_ix} в объекте «{source_object}» ({source_dir:?})"
                                ),
                                None,
                            );
                            app.drag.state = DragInteractionState::Idle;
                            return;
                        }
                    }
                }
            }
            // Соединение показателя с объектом-целью (существующая логика).
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

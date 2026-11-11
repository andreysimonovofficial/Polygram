use crate::app::PolygramApp;
use polygram_canvas::{department_at, object_at};
use polygram_core::geometry::Geometry;
use polygram_core::layout::Rect as CoreRect;
use polygram_core::types::{Direction, NodeKind};
use polygram_core::{Command, MetricValue};
use egui::{Context, Pos2, Rect, Vec2, Response, PointerButton};

/// Вся логика перетаскивания: старт (объект / показатель / selection box),
/// клиентское предсказание позиции, призрачная копия, дроп объекта и показателя,
/// завершение выделения рамкой.
pub fn handle_drag(
    app: &mut PolygramApp,
    ctx: &Context,
    ui: &egui::Ui,
    canvas_response: &Response,
    geometry: &Geometry,
) {
    // Перемещение объекта за шапку / перетаскивание показателя / начало selection box.
    if canvas_response.drag_started_by(PointerButton::Primary)
        && app.drag.drag_metric.is_none()
    {
        if let Some(pos) = canvas_response.interact_pointer_pos() {
            // Проверяем, начато ли перетаскивание из строки показателя.
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
                    app.drag.drag_metric =
                        Some((object_id.clone(), *dir, entry));
                    app.drag.drag_metric_start_screen = Some(pos);
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
                    app.drag.drag_object = Some(object_id.to_owned());
                    app.drag.drag_start_screen = Some(pos);
                    app.drag.drag_start_cell = Some((o.rect.x, o.rect.y));
                    if o.kind != NodeKind::Department {
                        app.drag.drag_department =
                            department_at(geometry, pos, app.canvas.camera.offset, app.canvas.camera.zoom, app.canvas.cell_size)
                                .map(|s| s.to_owned());
                    }
                }
            } else {
                app.drag.selection_box_start = Some(pos);
            }
        }
    }

    // Перетаскивание объекта: клиентское предсказание позиции и призрачная копия.
    if let (Some(ref object_id), Some(start_screen), Some((start_x, start_y))) =
        (&app.drag.drag_object, app.drag.drag_start_screen, app.drag.drag_start_cell)
    {
        if canvas_response.dragged_by(PointerButton::Primary) {
            if let Some(cur) = canvas_response.interact_pointer_pos() {
                let cell_px = app.canvas.cell_px();
                let offset = app.canvas.camera.offset;
                let zoom = app.canvas.camera.zoom;
                let target_x = ((cur.x / zoom - offset.x) / app.canvas.cell_size).round() as i32;
                let target_y = ((cur.y / zoom - offset.y) / app.canvas.cell_size).round() as i32;
                let target = (target_x, target_y);
                let dragged_size = geometry
                    .objects
                    .iter()
                    .find(|o| o.id == *object_id)
                    .map(|o| (o.rect.w, o.rect.h));
                let proposed_rect = dragged_size.map(|(w, h)| {
                    CoreRect::new(target.0, target.1, w, h)
                });
                let overlap_obj = proposed_rect.and_then(|pr| {
                    geometry.objects.iter().find(|o| {
                        o.id != *object_id && o.rect.overlaps(&pr)
                    }).map(|o| (o.id.clone(), o.kind))
                });
                let (ghost, over) = match overlap_obj {
                    Some((dept_id, NodeKind::Department)) => {
                        let (w, h) = dragged_size.expect("dragged_size must be Some");
                        let g = polygram_canvas::find_ghost_cell_in_dept(&geometry, &dept_id, w, h, 2, object_id);
                        (g, Some(dept_id))
                    }
                    Some((id, _)) => {
                        let obj_b = geometry.objects.iter().find(|o| o.id == id)
                            .expect("overlap object must exist");
                        let (w, h) = dragged_size.expect("dragged_size must be Some");
                        let g = polygram_canvas::find_ghost_cell(&geometry, obj_b, w, h, 2, object_id, None);
                        (g, Some(id))
                    }
                    None => (None, None),
                };
                let can_move = ghost.is_some() || over.is_none();
                if can_move {
                    app.drag.predicted_positions.insert(object_id.clone(), target);
                }
                app.drag.drag_ghost = ghost;
                app.drag.drag_over = over;
            }
        }
    }

    // Завершение перетаскивания объекта при отпускании левой кнопки — команда на сервер.
    if app.drag.drag_object.is_some()
        && !ui.input(|i| i.pointer.button_down(PointerButton::Primary))
    {
        let drag_object = app.drag.drag_object.take();
        let drag_start_cell = app.drag.drag_start_cell.take();
        app.drag.drag_start_screen = None;

        if let (Some(object_id), Some((start_x, start_y))) = (drag_object, drag_start_cell) {
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
            let drop_cell = app.drag.drag_ghost.unwrap_or(predicted);
            let target_overlaps = dragged_size.is_some_and(|(w, h)| {
                let pr = CoreRect::new(drop_cell.0, drop_cell.1, w, h);
                geometry.objects.iter().any(|o| {
                    o.id != object_id && o.rect.overlaps(&pr)
                })
            });
            let skip_move = app.drag.drag_over.is_some() && app.drag.drag_ghost.is_none();
            if !skip_move && !target_overlaps {
                let pos_changed = (drop_cell.0, drop_cell.1) != (start_x, start_y);
                let dropped_on_dept = canvas_response
                    .interact_pointer_pos()
                    .and_then(|cur| {
                        department_at(geometry, cur, app.canvas.camera.offset, app.canvas.camera.zoom, app.canvas.cell_size)
                    })
                    .map_or(false, |dept_id| dept_id != object_id);

                if dropped_on_dept {
                    let dept_id = canvas_response
                        .interact_pointer_pos()
                        .and_then(|cur| {
                            department_at(geometry, cur, app.canvas.camera.offset, app.canvas.camera.zoom, app.canvas.cell_size)
                        })
                        .unwrap();
                    app.drag.pending_move =
                        Some((object_id.clone(), app.connection.last_rev));
                    app.apply_batch(vec![
                        Command::SetObjectParent {
                            object: object_id.clone(),
                            parent: Some(dept_id.to_owned()),
                        },
                        Command::MoveObject {
                            object: object_id.clone(),
                            at: [drop_cell.0, drop_cell.1],
                        },
                    ]);
                    app.selected_objects = vec![object_id.clone()];
                } else {
                    app.drag.drag_department = None;
                    if pos_changed {
                        app.drag.pending_move =
                            Some((object_id.clone(), app.connection.last_rev));
                        app.apply_cmd(Command::MoveObject {
                            object: object_id,
                            at: [drop_cell.0, drop_cell.1],
                        });
                    }
                }
            } else {
                app.drag.predicted_positions.remove(&object_id);
            }
        }
    }

    // Завершение перетаскивания показателя: копируем показатель в целевой объект и строим связь.
    if app.drag.drag_metric.is_some()
        && !ui.input(|i| i.pointer.button_down(PointerButton::Primary))
    {
        let (source_object, source_dir, entry) =
            app.drag.drag_metric.take().expect("drag_metric is Some");
        let metric_name = entry.name.clone();
        app.drag.drag_metric_start_screen = None;
        if let Some(pos) = canvas_response.interact_pointer_pos() {
            if let Some(target_id) = app
                .canvas
                .object_at(&geometry, pos)
                .map(|s| s.to_owned())
            {
                if target_id != source_object {
                    if let Some(target) = geometry.objects.iter().find(|o| o.id == target_id) {
                        if target.kind == NodeKind::Process {
                            let target_dir = match source_dir {
                                Direction::Request => Direction::Output,
                                Direction::Output => Direction::Request,
                            };
                            let entries = match target_dir {
                                Direction::Request => &target.requests,
                                Direction::Output => &target.outputs,
                            };
                            let already_has = entries.iter().any(|e| e.name == metric_name);
                            let mut cmds: Vec<Command> = Vec::new();
                            if !already_has {
                                cmds.push(Command::AddMetricEntry {
                                    object: target_id.clone(),
                                    dir: target_dir,
                                    id: Some(metric_name.clone()),
                                    metric_def: metric_name.clone(),
                                    name: metric_name.clone(),
                                    value: MetricValue::Null,
                                    required: MetricValue::Null,
                                    max: None,
                                    fact: MetricValue::Null,
                                });
                            }
                            let (from_object, from_entry, to_object, to_entry) =
                                match source_dir {
                                    Direction::Output => (
                                        source_object.clone(),
                                        metric_name.clone(),
                                        target_id.clone(),
                                        metric_name.clone(),
                                    ),
                                    Direction::Request => (
                                        target_id.clone(),
                                        metric_name.clone(),
                                        source_object.clone(),
                                        metric_name.clone(),
                                    ),
                                };
                            cmds.push(Command::Connect {
                                from_object,
                                from_entry,
                                to_object,
                                to_entry,
                                throughput: 100.0,
                            });
                            app.apply_batch(cmds);
                        }
                    }
                }
            }
        }
    }

    // Завершение выделения рамкой (Selection Box) — выделяем все объекты внутри прямоугольника.
    if app.drag.selection_box_start.is_some()
        && !ui.input(|i| i.pointer.button_down(PointerButton::Primary))
    {
        if let (Some(start), Some(end)) = (app.drag.selection_box_start, canvas_response.interact_pointer_pos()) {
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
            app.drag.selection_box_start = None;
        }
    }
}

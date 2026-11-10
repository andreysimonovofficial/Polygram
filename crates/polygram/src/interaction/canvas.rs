use crate::app::PolygramApp;
use polygram_canvas::{object_at, MetricBinding};
use polygram_core::types::{Direction, MAX_UI_METRICS_PER_SIDE, MetricKind, NodeKind};
use polygram_core::{Command, Published};
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
                app.drag.drag_object = None;
                app.drag.drag_start_screen = None;
                app.drag.drag_start_cell = None;
                app.dialogs.edit_target = Some(object_id.to_owned());
                app.dialogs.edit_input = obj_name;
            }
        }
    }

    // Камера: колёсико — зум к курсору, правая/средняя кнопка — панорамирование.
    if canvas_response.hovered() {
        // Зум колёсиком мыши к курсору (как в инструменте дизайна).
        let scroll = ctx.input(|i| i.raw_scroll_delta);
        if scroll.y.abs() > 0.001 {
            if let Some(cursor) = ctx.pointer_hover_pos() {
                let old_zoom = app.canvas.camera.zoom;
                let cell_size = app.canvas.cell_size;
                let offset = app.canvas.camera.offset;
                // Мировая точка (в клетках) под курсором до зума.
                let world_cursor = Pos2::new(
                    (cursor.x / old_zoom - offset.x) / cell_size,
                    (cursor.y / old_zoom - offset.y) / cell_size,
                );
                let factor = (scroll.y * 0.01).exp();
                let new_zoom = (old_zoom * factor).clamp(0.1, 10.0);
                if new_zoom != old_zoom {
                    // Новое смещение, чтобы та же мировая точка осталась под курсором.
                    app.canvas.camera.offset.x = cursor.x / new_zoom - world_cursor.x * cell_size;
                    app.canvas.camera.offset.y = cursor.y / new_zoom - world_cursor.y * cell_size;
                    app.canvas.camera.zoom = new_zoom;
                    ctx.request_repaint();
                }
            }
        }
        // Панорамирование правой или средней кнопкой.
        if canvas_response.dragged_by(egui::PointerButton::Secondary)
            || canvas_response.dragged_by(egui::PointerButton::Middle)
        {
            let cell_size = app.canvas.cell_size;

            app.canvas.camera.offset += ui.input(|i| i.pointer.delta());
            app.canvas.camera.offset.x =
                (app.canvas.camera.offset.x / cell_size).round() * cell_size;
            app.canvas.camera.offset.y =
                (app.canvas.camera.offset.y / cell_size).round() * cell_size;
            ctx.request_repaint();
        }

        // Выделение объектов по клику (только если не перетаскивание и не selection box).
        if canvas_response.clicked()
            && app.drag.drag_object.is_none()
            && app.drag.drag_metric.is_none()
            && app.drag.selection_box_start.is_none()
        {
            if let Some(pos) = canvas_response.interact_pointer_pos() {
                let object_id = object_at(
                    &geometry,
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
        }

        // Перемещение объекта за шапку: зажать левую кнопку на шапке и тащить.
        // Перетаскивание показателя: зажать левую кнопку на строке показателя и тащить в другой объект.
        if canvas_response.drag_started_by(egui::PointerButton::Primary)
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
                    app.drag.drag_metric =
                        Some((object_id.clone(), *dir, metric_name.clone()));
                    app.drag.drag_metric_start_screen = Some(pos);
                } else if let Some(object_id) = object_at(
                    &geometry,
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
                    }
                } else {
                    // Selection Box: начинаем выделение рамкой из пустого места.
                    app.drag.selection_box_start = Some(pos);
                }
            }
        }

        // Перетаскивание: предсказываем позицию мгновенно (client-side prediction),
        // команду на сервер отправляем только при отпускании мыши.
        if let (Some(ref object_id), Some(start_screen), Some((start_x, start_y))) =
            (&app.drag.drag_object, app.drag.drag_start_screen, app.drag.drag_start_cell)
        {
            if canvas_response.dragged_by(egui::PointerButton::Primary) {
                if let Some(cur) = canvas_response.interact_pointer_pos() {
                    let delta = cur - start_screen;
                    let cell_px = app.canvas.cell_px();
                    let dx = (delta.x / cell_px).round() as i32;
                    let dy = (delta.y / cell_px).round() as i32;
                    let target = [start_x + dx, start_y + dy];
                    // Клиентская предсказание: объект двигается мгновенно, без ожидания сервера.
                    app.drag.predicted_positions.insert(object_id.clone(), (target[0], target[1]));
                }
            }
        } else if let Some((source_object, source_dir, metric_name)) = &app.drag.drag_metric {
            // Предварительный просмотр перетаскивания показателя: подпись у курсора и подсветка целевого объекта.
            if canvas_response.dragged_by(egui::PointerButton::Primary) {
                if let Some(cur) = canvas_response.interact_pointer_pos() {
                    let painter = ui.painter();
                    painter.text(
                        cur + Vec2::new(14.0, 14.0),
                        egui::Align2::LEFT_TOP,
                        format!("⇨ {}", metric_name),
                        egui::FontId::new(16.0, egui::FontFamily::Proportional),
                        Color32::from_rgb(0, 217, 255),
                    );
                    // Подсвечиваем целевой объект под курсором (процесс, не источник).
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
                                    app.canvas
                                        .camera
                                        .world_to_screen(Pos2::new(r.x as f32, r.y as f32), app.canvas.cell_size),
                                    Vec2::new(r.w as f32 * app.canvas.cell_px(), r.h as f32 * app.canvas.cell_px()),
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
        } else if let Some(start) = app.drag.selection_box_start {
            // Selection Box: рисуем резиновый прямоугольник во время перетаскивания.
            if canvas_response.dragged_by(egui::PointerButton::Primary) {
                if let Some(cur) = canvas_response.interact_pointer_pos() {
                    let painter = ui.painter();
                    let x = start.x.min(cur.x);
                    let y = start.y.min(cur.y);
                    let w = (cur.x - start.x).abs();
                    let h = (cur.y - start.y).abs();
                    if w > 1.0 && h > 1.0 {
                        let box_rect = Rect::from_min_size(
                            Pos2::new(x, y),
                            Vec2::new(w, h),
                        );
                        painter.rect_stroke(
                            box_rect,
                            0.0,
                            Stroke::new(2.0_f32, Color32::from_rgb(0, 217, 255)),
                        );
                    }
                }
            }
        }

        // Завершение перетаскивания при отпускании левой кнопки мыши — отправляем команду на сервер.
        if app.drag.drag_object.is_some()
            && !ui.input(|i| i.pointer.button_down(egui::PointerButton::Primary))
        {
            let drag_object = app.drag.drag_object.take();
            let drag_start_cell = app.drag.drag_start_cell.take();
            app.drag.drag_start_screen = None;
            if let (Some(object_id), Some((start_x, start_y))) = (drag_object, drag_start_cell) {
                let target = app.drag.predicted_positions.get(&object_id).copied().unwrap_or((start_x, start_y));
                // Отправляем на сервер только если позиция реально изменилась относительно начала перетаскивания.
                if (target.0, target.1) != (start_x, start_y) {
                    app.drag.pending_move = Some((object_id.clone(), app.connection.last_rev));
                    app.apply_cmd(Command::MoveObject {
                        object: object_id,
                        at: [target.0, target.1],
                    });
                }
                // Не очищаем predicted_positions здесь — оставим до подтверждения сервера (reconciliation в poll_snapshot),
                // иначе будет виден скачок на старую позицию.
            }
        }

        // Завершение перетаскивания показателя: копируем показатель в целевой объект (в противоположном направлении)
        // и строим связь между записями Выход → Запрос.
        if app.drag.drag_metric.is_some()
            && !ui.input(|i| i.pointer.button_down(egui::PointerButton::Primary))
        {
            let (source_object, source_dir, metric_name) =
                app.drag.drag_metric.take().expect("drag_metric is Some");
            app.drag.drag_metric_start_screen = None;
                if let Some(pos) = canvas_response.interact_pointer_pos() {
                    if let Some(target_id) = app
                        .canvas
                        .object_at(&geometry, pos)
                        .map(|s| s.to_owned())
                    {
                    if target_id != source_object {
                        if let Some(target) = geometry
                            .objects
                            .iter()
                            .find(|o| o.id == target_id)
                        {
                            if target.kind == NodeKind::Process {
                                // Целевое направление — противоположно источнику.
                                let target_dir = match source_dir {
                                    Direction::Request => Direction::Output,
                                    Direction::Output => Direction::Request,
                                };
                                let entries = match target_dir {
                                    Direction::Request => &target.requests,
                                    Direction::Output => &target.outputs,
                                };
                                let already_has =
                                    entries.iter().any(|e| e.name == metric_name);
                                let mut cmds: Vec<Command> = Vec::new();
                                if !already_has {
                                    // Добавляем тот же показатель в целевой объект (значение по умолчанию 0).
                                    cmds.push(Command::AddMetricEntry {
                                        object: target_id.clone(),
                                        dir: target_dir,
                                        id: Some(metric_name.clone()),
                                        metric_def: metric_name.clone(),
                                        name: metric_name.clone(),
                                        value: 0.0,
                                        required: 0.0,
                                        max: None,
                                        fact: 0.0,
                                    });
                                }
                                // Связь всегда Выход → Запрос.
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
            && !ui.input(|i| i.pointer.button_down(egui::PointerButton::Primary))
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
                        let r = polygram_core::layout::Rect::new(object.rect.x, object.rect.y, object.rect.w, object.rect.h);
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
            }
            app.drag.selection_box_start = None;
        }

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
    for (id, info) in &app.api_bindings {
        if let Some((object_id, entry_name)) = id.split_once(':') {
            bindings.insert(
                (object_id.to_owned(), entry_name.to_owned()),
                MetricBinding {
                    ok: info.ok,
                    last_updated: info.last_updated.clone(),
                },
            );
        }
    }

    app.canvas.paint(ui, &app.geometry_with_predictions(&published), &published.dynamic, hovered_object, app.dialogs.edit_target.as_deref(), &app.selected_objects, app.show_grid, &bindings);

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
                app.dialogs.add_metric_kind = MetricKind::Number;
                app.dialogs.add_metric_value = "0".to_owned();
                app.dialogs.add_metric_unit = String::new();
                app.drag.drag_object = None;
                app.drag.drag_start_screen = None;
                app.drag.drag_start_cell = None;
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
                app.drag.drag_object = None;
                app.drag.drag_start_screen = None;
                app.drag.drag_start_cell = None;
                break;
            }
        }
    }

    super::container_menu::handle_container_menu_dialog(app, ctx, published);
}

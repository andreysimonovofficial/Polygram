use crate::app::PolygramApp;
use polygram_canvas::object_at;
use polygram_core::types::{Direction, MAX_UI_METRICS_PER_SIDE, NodeKind};
use polygram_core::{Command, MetricKind, Published};
use egui::{Color32, Key, Pos2, Rect, Stroke, Vec2};

pub fn context_menu(
    app: &mut PolygramApp,
    ctx: &egui::Context,
    published: &polygram_core::Published,
    close_menu: &mut bool,
) {
    let menu_data = app.right_click.clone();
    if let Some((pos, object_id)) = &menu_data {
        egui::Window::new("Контекстное меню")
            .default_pos(*pos)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                if let Some(object_id) = object_id {
                    if ui.button(format!("Удалить объект: {object_id}")).clicked() {
                        app.dialogs.delete_confirm = Some(vec![object_id.clone()]);
                        *close_menu = true;
                    }
                    // Групповое удаление выделенных объектов (если выбрано несколько)
                    if app.selected_objects.len() > 1 {
                        if ui.button(format!("Удалить выделенные ({})", app.selected_objects.len())).clicked() {
                            app.dialogs.delete_confirm = Some(app.selected_objects.clone());
                            *close_menu = true;
                        }
                    }
                    if ui.button("Добавить показатель в запрос").clicked()
                    {
                        let n_req = published
                            .geometry
                            .objects
                            .iter()
                            .find(|n| n.id == *object_id)
                            .map(|n| n.requests.len())
                            .unwrap_or(0);
                        let cmd = Command::AddMetricEntry {
                            object: object_id.clone(),
                            dir: Direction::Request,
                            id: None,
                            metric_def: "по умолчанию".into(),
                            name: format!("request{}", n_req + 1),
                            value: 0.0,
                            required: 0.0,
                            max: None,
                            fact: 0.0,
                        };
                        app.apply_cmd(cmd);
                        *close_menu = true;
                    }
                    if ui.button("Добавить показатель в выход").clicked() {
                        let n_out = published
                            .geometry
                            .objects
                            .iter()
                            .find(|n| n.id == *object_id)
                            .map(|n| n.outputs.len())
                            .unwrap_or(0);
                        let cmd = Command::AddMetricEntry {
                            object: object_id.clone(),
                            dir: Direction::Output,
                            id: None,
                            metric_def: "по умолчанию".into(),
                            name: format!("out{}", n_out + 1),
                            value: 0.0,
                            required: 0.0,
                            max: None,
                            fact: 0.0,
                        };
                        app.apply_cmd(cmd);
                        *close_menu = true;
                    }
                    ui.separator();
                }
                if let Some(object_id) = object_id {
                    let obj_kind = published
                        .geometry
                        .objects
                        .iter()
                        .find(|n| n.id == *object_id)
                        .map(|n| n.kind);
                    if let Some(NodeKind::Department) = obj_kind {
                        if ui.button("Добавить объект в отдел").clicked() {
                            let world_pos = app
                                .canvas
                                .camera
                                .screen_to_world(*pos, app.canvas.cell_size);
                            let cmd = Command::AddObject {
                                id: None,
                                name: "Новый процесс".to_owned(),
                                kind: NodeKind::Process,
                                at: Some([world_pos.x.round() as i32, world_pos.y.round() as i32]),
                                parent: Some(object_id.clone()),
                                author: None,
                            };
                            app.apply_cmd(cmd);
                            *close_menu = true;
                        }
                        if ui.button("Добавить отдел в отдел").clicked() {
                            let world_pos = app
                                .canvas
                                .camera
                                .screen_to_world(*pos, app.canvas.cell_size);
                            let cmd = Command::AddObject {
                                id: None,
                                name: "Новый отдел".to_owned(),
                                kind: NodeKind::Department,
                                at: Some([world_pos.x.round() as i32, world_pos.y.round() as i32]),
                                parent: Some(object_id.clone()),
                                author: None,
                            };
                            app.apply_cmd(cmd);
                            *close_menu = true;
                        }
                        ui.separator();
                    }
                    if ui.button("Переименовать объект").clicked() {
                        let obj_name = published
                            .geometry
                            .objects
                            .iter()
                            .find(|n| n.id == *object_id)
                            .map(|n| n.name.clone())
                            .unwrap_or_else(|| object_id.to_string());
                        app.dialogs.rename_target = Some(object_id.clone());
                        app.dialogs.rename_input = obj_name;
                        *close_menu = true;
                    }
                }
                if ui.button("Автораскладка").clicked() {
                    app.apply_cmd(Command::AutoLayout);
                    *close_menu = true;
                }
                if ui.button("Перемаршрутизировать").clicked() {
                    app.apply_cmd(Command::RerouteAll);
                    *close_menu = true;
                }
                if object_id.is_none() {
                    if ui.button("Создать Процесс").clicked() {
                        let world_pos = app
                            .canvas
                            .camera
                            .screen_to_world(*pos, app.canvas.cell_px());
                        let cmd = Command::AddObject {
                            id: None,
                            name: "Процесс".to_owned(),
                            kind: NodeKind::Process,
                            at: Some([world_pos.x.round() as i32, world_pos.y.round() as i32]),
                            parent: None,
                            author: None,
                        };
                        app.apply_cmd(cmd);
                        *close_menu = true;
                    }
                    if ui.button("Создать отдел").clicked() {
                        let world_pos = app
                            .canvas
                            .camera
                            .screen_to_world(*pos, app.canvas.cell_px());
                        let cmd = Command::AddObject {
                            id: None,
                            name: "Новый отдел".to_owned(),
                            kind: NodeKind::Department,
                            at: Some([world_pos.x as i32, world_pos.y as i32]),
                            parent: None,
                            author: None,
                        };
                        app.apply_cmd(cmd);
                        *close_menu = true;
                    }
                }
            });
    }
    if *close_menu {
        app.right_click = None;
    }
}

/// Подтверждение удаления объектов ("Удалить?").
/// Показывает диалог со списком объектов на удаление. При подтверждении
/// отправляет команды RemoveObject для каждого объекта и очищает выделение.
pub fn handle_delete_confirm(app: &mut PolygramApp, ctx: &egui::Context) {
    let targets = match app.dialogs.delete_confirm.clone() {
        Some(t) if !t.is_empty() => t,
        _ => return,
    };

    let mut do_delete = false;
    let mut do_cancel = false;

    let count = targets.len();
    egui::Window::new("Удалить?")
        .default_pos(egui::pos2(400.0, 300.0))
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label(format!("Удалить {} объект{}?", count, if count == 1 { "" } else { "а" }));
            ui.separator();
            for id in &targets {
                ui.label(id);
            }
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Да").clicked() {
                    do_delete = true;
                }
                if ui.button("Нет").clicked() {
                    do_cancel = true;
                }
            });
        });

    if do_cancel {
        app.dialogs.delete_confirm = None;
        return;
    }

    if do_delete {
        for id in &targets {
            app.apply_cmd(Command::RemoveObject { object: id.clone() });
        }
        // Удаляем удалённые объекты из выделения.
        app.selected_objects.retain(|s| !targets.iter().any(|t| t == s));
        app.dialogs.delete_confirm = None;
    }
}

pub fn handle_canvas_interaction(
    app: &mut PolygramApp,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    published: &polygram_core::Published,
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

    app.canvas.paint(ui, &app.geometry_with_predictions(&published), &published.dynamic, hovered_object, app.dialogs.edit_target.as_deref(), &app.selected_objects);

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

    handle_add_metric_dialog(app, ctx, &published);

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

    handle_container_menu_dialog(app, ctx, published);
}

pub fn handle_container_menu_dialog(
    app: &mut PolygramApp,
    ctx: &egui::Context,
    published: &polygram_core::Published,
) {
    let Some((object_id, dir)) = &app.dialogs.container_menu_target else {
        return;
    };
    let object_id = object_id.clone();
    let dir = *dir;

    let label = match dir {
        Direction::Request => "ЗАПРОС",
        Direction::Output => "ВЫХОД",
    };

    let obj = match published
        .geometry
        .objects
        .iter()
        .find(|o| o.id == *object_id)
    {
        Some(o) => o,
        None => {
            app.dialogs.container_menu_target = None;
            return;
        }
    };

    let entries: Vec<&polygram_core::geometry::EntryGeom> = match dir {
        Direction::Request => obj.requests.iter().collect(),
        Direction::Output => obj.outputs.iter().collect(),
    };

    // Инициализировать буферы редактирования (имя, значение, единица).
    if app.dialogs.container_menu_edits.is_empty() {
        for entry in &entries {
            app.dialogs.container_menu_edits.push((
                entry.name.clone(),
                format!("{:.0}", entry.value),
                entry.unit.clone(),
            ));
        }
    } else {
        let entry_names: std::collections::HashSet<&str> =
            entries.iter().map(|e| e.name.as_str()).collect();
        app.dialogs.container_menu_edits
            .retain(|(n, _, _)| entry_names.contains(n.as_str()));
        for entry in &entries {
            if !app
                .dialogs.container_menu_edits
                .iter()
                .any(|(n, _, _)| *n == entry.name)
            {
                app.dialogs.container_menu_edits.push((
                    entry.name.clone(),
                    format!("{:.0}", entry.value),
                    entry.unit.clone(),
                ));
            }
        }
    }

    let mut delete_entry: Option<String> = None;
    let mut apply_edits = false;
    let mut close_dialog = false;

    egui::Window::new(format!("Показатели — {label} ({object_id})"))
        .default_size([500.0, 400.0])
        .resizable(true)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label(format!("{label} — {}", entries.len()));
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                for entry in entries.iter() {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(&entry.name);
                        ui.label("значение:");

                        let buf_idx = app
                            .dialogs.container_menu_edits
                            .iter()
                            .position(|(n, _, _)| *n == entry.name);
                        if buf_idx.is_none() {
                            app.dialogs.container_menu_edits.push((
                                entry.name.clone(),
                                format!("{:.0}", entry.value),
                                entry.unit.clone(),
                            ));
                        }
                        let buf_idx = app
                            .dialogs.container_menu_edits
                            .iter()
                            .position(|(n, _, _)| *n == entry.name)
                            .unwrap();
                        let (value_buf, unit_buf) = {
                            let e = &mut app.dialogs.container_menu_edits[buf_idx];
                            (&mut e.1, &mut e.2)
                        };

                        ui.text_edit_singleline(value_buf);
                        ui.label("единица:");
                        ui.text_edit_singleline(unit_buf);

                        if ui.button("🗑").clicked() {
                            delete_entry = Some(entry.name.clone());
                        }
                    });
                }
            });

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Применить").clicked() {
                    apply_edits = true;
                }
                if ui.button("Закрыть").clicked() {
                    close_dialog = true;
                }
            });
        });

    // Применить изменения значений и единиц измерения.
    if apply_edits {
        let value_cmds: Vec<(String, f64)> = app
            .dialogs.container_menu_edits
            .iter()
            .filter_map(|(entry_name, value_str, _)| {
                let value: f64 = value_str.trim().parse().unwrap_or(0.0);
                Some((entry_name.clone(), value))
            })
            .collect();
        let unit_cmds: Vec<(String, String)> = app
            .dialogs.container_menu_edits
            .iter()
            .filter_map(|(entry_name, _, unit_str)| {
                let unit = unit_str.trim().to_owned();
                if !unit.is_empty() {
                    Some((entry_name.clone(), unit))
                } else {
                    None
                }
            })
            .collect();
        app.dialogs.container_menu_edits.clear();
        for (entry_name, value) in value_cmds {
            app.apply_cmd(polygram_core::Command::SetValue {
                object: object_id.clone(),
                entry: entry_name,
                value,
            });
        }
        for (metric_def, unit) in unit_cmds {
            app.apply_cmd(polygram_core::Command::SetMetricDefUnit {
                metric_def,
                unit,
            });
        }
    }

    // Удалить показатель.
    if let Some(entry_name) = delete_entry {
        app.apply_cmd(polygram_core::Command::RemoveMetricEntry {
            object: object_id.clone(),
            entry: entry_name,
        });
    }

    if close_dialog {
        app.dialogs.container_menu_target = None;
        app.dialogs.container_menu_edits.clear();
    }
}

pub fn handle_rename_dialog(app: &mut PolygramApp, ctx: &egui::Context) {
    let rename_target = app.dialogs.rename_target.clone();
    let mut do_rename = false;
    if let Some(target) = &rename_target {
        egui::Window::new("Переименовать объект")
            .default_pos(egui::pos2(400.0, 300.0))
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label(format!("Объект: {target}"));
                ui.text_edit_singleline(&mut app.dialogs.rename_input);
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        do_rename = true;
                    }
                    if ui.button("Отмена").clicked() {
                        app.dialogs.rename_target = None;
                        app.dialogs.rename_input.clear();
                    }
                });
            });
    }
    if do_rename && !app.dialogs.rename_input.is_empty() {
        app.apply_cmd(polygram_core::Command::RenameObject {
            object: rename_target.unwrap(),
            name: app.dialogs.rename_input.clone(),
        });
        app.dialogs.rename_target = None;
        app.dialogs.rename_input.clear();
    }
}

/// Инлайн-редактирование названия объекта прямо на холсте (Figma-стиль).
/// Поле ввода рисуется точно поверх текста названия (shape "ui-1").
pub fn inline_rename_editor(
    app: &mut PolygramApp,
    ctx: &egui::Context,
    published: &polygram_core::Published,
) {
    let Some(target) = &app.dialogs.edit_target else {
        return;
    };
    let geometry = app.geometry_with_predictions(published);
    let Some(object) = geometry.objects.iter().find(|o| o.id == target.as_str()) else {
        app.dialogs.edit_target = None;
        return;
    };

    let cell_px = app.canvas.cell_px();
    let zoom = app.canvas.camera.zoom;
    let theme = &app.canvas.theme;
    let screen_min = app.canvas.camera.world_to_screen(
        Pos2::new(object.rect.x as f32, object.rect.y as f32),
        app.canvas.cell_size,
    );
    let header_h_px = cell_px * 1.0; // OBJECT_HEADER_HEIGHT_CELLS = 1.0
    let edit_rect = Rect::from_min_size(
        Pos2::new(
            screen_min.x + 16.0 * zoom,
            screen_min.y + header_h_px / 2.0 - (theme.text_size * zoom) / 2.0,
        ),
        Vec2::new(
            (object.rect.w as f32 * cell_px - 32.0 * zoom).max(60.0),
            theme.text_size * zoom,
        ),
    );

    let mut commit = false;
    let mut cancel = false;

    egui::Area::new(egui::Id::new("inline_rename"))
        .order(egui::Order::Foreground)
        .fixed_pos(edit_rect.min)
        .show(ctx, |ui| {
            ui.set_min_width(edit_rect.width().max(40.0));
            ui.set_min_height(edit_rect.height().max(16.0));
            egui::Frame::none().show(ui, |ui| {
                let response = ui.text_edit_singleline(&mut app.dialogs.edit_input);
                response.request_focus();

                if response.has_focus() {
                    if ctx.input(|i| i.key_pressed(Key::Enter)) {
                        commit = true;
                    }
                    if ctx.input(|i| i.key_pressed(Key::Escape)) {
                        cancel = true;
                    }
                }
                if response.lost_focus() {
                    commit = true;
                }
            });
        });

    if commit {
        if !app.dialogs.edit_input.is_empty() {
            app.apply_cmd(polygram_core::Command::RenameObject {
                object: target.clone(),
                name: app.dialogs.edit_input.clone(),
            });
        }
        app.dialogs.edit_target = None;
        app.dialogs.edit_input.clear();
    } else if cancel {
        app.dialogs.edit_target = None;
        app.dialogs.edit_input.clear();
    }
}

/// Модальное диалоговое окно добавления показателя.
pub fn handle_add_metric_dialog(
    app: &mut PolygramApp,
    ctx: &egui::Context,
    published: &Published,
) {
    let Some((object_id, dir)) = &app.dialogs.add_metric_target else {
        return;
    };
    let object_id = object_id.clone();
    let dir = *dir;

    let label = match dir {
        Direction::Request => "ЗАПРОС",
        Direction::Output => "ВЫХОД",
    };
    let value_label = match dir {
        Direction::Request => "Необходимое (required):",
        Direction::Output => "Значение (value):",
    };

    let mut done = false;
    let mut cancelled = false;

    // Ищем существующий показатель по введённому имени (для подстановки данных).
    let existing_md = published
        .metric_defs
        .iter()
        .find(|md| md.name == app.dialogs.add_metric_name.trim())
        .cloned();

    egui::Window::new(format!("Добавить показатель — {label}"))
        .default_size([320.0, 240.0])
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label("Имя показателя:");
            ui.text_edit_singleline(&mut app.dialogs.add_metric_name);

            if let Some(md) = &existing_md {
                ui.label(format!("Найден существующий показатель: {}", md.name));
                ui.label(format!("Тип: {:?}", md.kind));
                ui.label(format!("Единица: {}", md.unit));
                ui.separator();
                ui.label(value_label);
                ui.text_edit_singleline(&mut app.dialogs.add_metric_value);
            } else {
                ui.separator();
                ui.label("Тип:");
                egui::ComboBox::new("add_metric_kind", "")
                    .selected_text(format!("{:?}", app.dialogs.add_metric_kind))
                    .show_ui(ui, |ui| {
                        for kind in [
                            MetricKind::Number,
                            MetricKind::Time,
                            MetricKind::Percentage,
                            MetricKind::Binary,
                            MetricKind::Duration,
                            MetricKind::Formula,
                        ] {
                            if ui.selectable_label(app.dialogs.add_metric_kind == kind, format!("{:?}", kind)).clicked() {
                                app.dialogs.add_metric_kind = kind;
                            }
                        }
                    });

                ui.separator();
                ui.label(value_label);
                ui.text_edit_singleline(&mut app.dialogs.add_metric_value);

                ui.separator();
                ui.label("Единица измерения:");
                ui.text_edit_singleline(&mut app.dialogs.add_metric_unit);
            }

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    done = true;
                }
                if ui.button("Отмена").clicked() {
                    cancelled = true;
                }
            });
        });

    if cancelled {
        app.dialogs.add_metric_target = None;
        app.dialogs.add_metric_name.clear();
        app.dialogs.add_metric_value.clear();
        app.dialogs.add_metric_unit.clear();
        return;
    }

    if done {
        let name = app.dialogs.add_metric_name.trim().to_owned();
        if name.is_empty() {
            app.dialogs.add_metric_target = None;
            app.dialogs.add_metric_name.clear();
            app.dialogs.add_metric_value.clear();
            app.dialogs.add_metric_unit.clear();
            return;
        }

        let value: f64 = app.dialogs.add_metric_value.trim().parse().unwrap_or(0.0);

        if let Some(md) = &existing_md {
            // Используем существующий показатель — создаём только запись на объекте.
            // Для запроса вводимое значение — необходимое (required), для выхода — значение (value).
            let (entry_value, entry_required) = match dir {
                Direction::Request => (0.0, value),
                Direction::Output => (value, 0.0),
            };
            app.apply_batch(vec![Command::AddMetricEntry {
                object: object_id,
                dir,
                id: Some(name.clone()),
                metric_def: md.name.clone(),
                name,
                value: entry_value,
                required: entry_required,
                max: None,
                fact: 0.0,
            }]);
        } else {
            let kind = app.dialogs.add_metric_kind;
            let unit = app.dialogs.add_metric_unit.trim().to_owned();
            let (entry_value, entry_required) = match dir {
                Direction::Request => (0.0, value),
                Direction::Output => (value, 0.0),
            };

            app.apply_batch(vec![
                Command::AddMetricDef {
                    id: Some(name.clone()),
                    name: name.clone(),
                    kind,
                    unit,
                    api: None,
                },
                Command::AddMetricEntry {
                    object: object_id,
                    dir,
                    id: Some(name.clone()),
                    metric_def: name.clone(),
                    name,
                    value: entry_value,
                    required: entry_required,
                    max: None,
                    fact: 0.0,
                },
            ]);
        }

        app.dialogs.add_metric_target = None;
        app.dialogs.add_metric_name.clear();
        app.dialogs.add_metric_kind = MetricKind::Number;
        app.dialogs.add_metric_value.clear();
        app.dialogs.add_metric_unit.clear();
    }
}

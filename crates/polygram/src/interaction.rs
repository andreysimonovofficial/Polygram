use crate::app::PolygramApp;
use polygram_canvas::object_at;
use polygram_core::types::{Direction, MAX_UI_METRICS_PER_SIDE, NodeKind};
use polygram_core::{Command, MetricKind};
use egui::{Key, Pos2, Rect, Vec2};

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
                        app.apply_cmd(Command::RemoveObject {
                            object: object_id.clone(),
                        });
                        *close_menu = true;
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
                        app.rename_target = Some(object_id.clone());
                        app.rename_input = obj_name;
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

    // Двойной клик по объекту — инлайн-редактирование названия прямо на холсте.
    if canvas_response.hovered() && ctx.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary)) {
        if let Some(pos) = canvas_response.interact_pointer_pos() {
            if let Some(object_id) = app.canvas.object_at(&published.geometry, pos) {
                let obj_name = published
                    .geometry
                    .objects
                    .iter()
                    .find(|o| o.id == object_id)
                    .map(|o| o.name.clone())
                    .unwrap_or_else(|| object_id.to_string());
                app.drag_object = None;
                app.drag_start_screen = None;
                app.drag_start_cell = None;
                app.edit_target = Some(object_id.to_owned());
                app.edit_input = obj_name;
            }
        }
    }

    // Камера: колесико — зум (приближение вверх, к курсору),
    // правая/средняя кнопка — панорамирование.
    if canvas_response.hovered() {
        // Зум через события колесика мыши (центрируется на курсору).
        let events: Vec<egui::Event> = ctx.input(|i| i.events.clone());
        for event in events.iter() {
            if let egui::Event::MouseWheel { delta, .. } = event {
                if delta.y != 0.0 {
                    // Приближение при прокрутке вверх (delta.y < 0).
                    let factor = (-delta.y * 0.05).exp();
                    let old_zoom = app.canvas.camera.zoom;
                    let new_zoom = (old_zoom * factor).clamp(0.1, 10.0);
                    if new_zoom != old_zoom {
                        if let Some(cursor) = canvas_response.interact_pointer_pos() {
                            let cell_size = app.canvas.cell_size;
                            // Мировая точка (в клетках) под курсором до зума.
                            let world_x = (cursor.x / old_zoom - app.canvas.camera.offset.x) / cell_size;
                            let world_y = (cursor.y / old_zoom - app.canvas.camera.offset.y) / cell_size;
                            // Новое смещение, чтобы та же мировая точка осталась под курсором.
                            // Округляем до кратных cell_size, чтобы объекты лежали на линиях сетки.
                            let cell_size = app.canvas.cell_size;
                            app.canvas.camera.offset.x =
                                ((cursor.x / new_zoom - world_x * cell_size) / cell_size).round() * cell_size;
                            app.canvas.camera.offset.y =
                                ((cursor.y / new_zoom - world_y * cell_size) / cell_size).round() * cell_size;
                        }
                        app.canvas.camera.zoom = new_zoom;
                        ctx.request_repaint();
                    }
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

        // Перемещение объекта за шапку: зажать левую кнопку на шапке и тащить.
        if canvas_response.drag_started_by(egui::PointerButton::Primary) {
            if let Some(pos) = canvas_response.interact_pointer_pos() {
                if let Some(object_id) = object_at(
                    &published.geometry,
                    pos,
                    app.canvas.camera.offset,
                    app.canvas.camera.zoom,
                    app.canvas.cell_size,
                ) {
                    let object = published
                        .geometry
                        .objects
                        .iter()
                        .find(|o| o.id == *object_id);
                    if let Some(o) = object {
                        app.drag_object = Some(object_id.to_owned());
                        app.drag_start_screen = Some(pos);
                        app.drag_start_cell = Some((o.rect.x, o.rect.y));
                    }
                }
            }
        }

        // Перетаскивание: пересчитываем целевую клетку и двигаем объект командой.
        if let (Some(ref object_id), Some(start_screen), Some((start_x, start_y))) =
            (&app.drag_object, app.drag_start_screen, app.drag_start_cell)
        {
            if canvas_response.dragged_by(egui::PointerButton::Primary) {
                if let Some(cur) = canvas_response.interact_pointer_pos() {
                    let delta = cur - start_screen;
                    let cell_px = app.canvas.cell_px();
                    let dx = (delta.x / cell_px).round() as i32;
                    let dy = (delta.y / cell_px).round() as i32;
                    let target = [start_x + dx, start_y + dy];
                    let current_cell = published
                        .geometry
                        .objects
                        .iter()
                        .find(|o| o.id == *object_id)
                        .map(|o| (o.rect.x, o.rect.y));
                    if current_cell != Some((target[0], target[1])) {
                        let _ = app.runtime.handle().apply(Command::MoveObject {
                            object: object_id.clone(),
                            at: target,
                        });
                    }
                }
            }
        }

        // Завершение перетаскивания при отпускании левой кнопки мыши.
        if app.drag_object.is_some()
            && !ui.input(|i| i.pointer.button_down(egui::PointerButton::Primary))
        {
            app.drag_object = None;
            app.drag_start_screen = None;
            app.drag_start_cell = None;
        }

        // Контекстное меню по правой кнопке (только клик, не перетаскивание).
        if canvas_response.secondary_clicked() {
            if let Some(pos) = canvas_response.interact_pointer_pos() {
                let object_id = app.canvas.object_at(&published.geometry, pos);
                app.right_click = Some((pos, object_id.map(|s| s.to_owned())));
            }
        }
    }

    // Ховер над объектом для подсветки рамки и текста.
    let hovered_object = if canvas_response.hovered() {
        if let Some(pos) = canvas_response.interact_pointer_pos() {
            app.canvas.object_at(&published.geometry, pos)
        } else {
            None
        }
    } else {
        None
    };

    app.canvas
        .paint(ui, &published.geometry, &published.dynamic, hovered_object, app.edit_target.as_deref());

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
                app.add_metric_target = Some((object_id.clone(), *dir));
                app.add_metric_name = match dir {
                    Direction::Request => format!("request{}", n + 1),
                    Direction::Output => format!("out{}", n + 1),
                };
                app.add_metric_kind = MetricKind::Number;
                app.add_metric_value = "0".to_owned();
                app.add_metric_unit = String::new();
                app.drag_object = None;
                app.drag_start_screen = None;
                app.drag_start_cell = None;
                break;
            }
        }
    }

    handle_add_metric_dialog(app, ctx);
}

pub fn handle_rename_dialog(app: &mut PolygramApp, ctx: &egui::Context) {
    let rename_target = app.rename_target.clone();
    let mut do_rename = false;
    if let Some(target) = &rename_target {
        egui::Window::new("Переименовать объект")
            .default_pos(egui::pos2(400.0, 300.0))
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label(format!("Объект: {target}"));
                ui.text_edit_singleline(&mut app.rename_input);
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        do_rename = true;
                    }
                    if ui.button("Отмена").clicked() {
                        app.rename_target = None;
                        app.rename_input.clear();
                    }
                });
            });
    }
    if do_rename && !app.rename_input.is_empty() {
        app.apply_cmd(polygram_core::Command::RenameObject {
            object: rename_target.unwrap(),
            name: app.rename_input.clone(),
        });
        app.rename_target = None;
        app.rename_input.clear();
    }
}

/// Инлайн-редактирование названия объекта прямо на холсте (Figma-стиль).
/// Поле ввода рисуется точно поверх текста названия (shape "ui-1").
pub fn inline_rename_editor(
    app: &mut PolygramApp,
    ctx: &egui::Context,
    published: &polygram_core::Published,
) {
    let Some(target) = &app.edit_target else {
        return;
    };
    let Some(object) = published.geometry.objects.iter().find(|o| o.id == target.as_str()) else {
        app.edit_target = None;
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
                let response = ui.text_edit_singleline(&mut app.edit_input);
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
        if !app.edit_input.is_empty() {
            app.apply_cmd(polygram_core::Command::RenameObject {
                object: target.clone(),
                name: app.edit_input.clone(),
            });
        }
        app.edit_target = None;
        app.edit_input.clear();
    } else if cancel {
        app.edit_target = None;
        app.edit_input.clear();
    }
}

/// Модальное диалоговое окно добавления показателя.
pub fn handle_add_metric_dialog(app: &mut PolygramApp, ctx: &egui::Context) {
    let Some((object_id, dir)) = &app.add_metric_target else {
        return;
    };
    let object_id = object_id.clone();
    let dir = *dir;

    let label = match dir {
        Direction::Request => "ЗАПРОС",
        Direction::Output => "ВЫХОД",
    };

    let mut done = false;
    let mut cancelled = false;

    egui::Window::new(format!("Добавить показатель — {label}"))
        .default_size([320.0, 240.0])
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label("Имя показателя:");
            ui.text_edit_singleline(&mut app.add_metric_name);

            ui.separator();
            ui.label("Тип:");
            egui::ComboBox::new("add_metric_kind", "")
                .selected_text(format!("{:?}", app.add_metric_kind))
                .show_ui(ui, |ui| {
                    for kind in [
                        MetricKind::Number,
                        MetricKind::Time,
                        MetricKind::Percentage,
                        MetricKind::Binary,
                        MetricKind::Duration,
                        MetricKind::Formula,
                    ] {
                        if ui.selectable_label(app.add_metric_kind == kind, format!("{:?}", kind)).clicked() {
                            app.add_metric_kind = kind;
                        }
                    }
                });

            ui.separator();
            ui.label("Значение:");
            ui.text_edit_singleline(&mut app.add_metric_value);

            ui.separator();
            ui.label("Единица измерения:");
            ui.text_edit_singleline(&mut app.add_metric_unit);

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
        app.add_metric_target = None;
        app.add_metric_name.clear();
        app.add_metric_value.clear();
        app.add_metric_unit.clear();
        return;
    }

    if done {
        let name = app.add_metric_name.trim().to_owned();
        if name.is_empty() {
            app.add_metric_target = None;
            app.add_metric_name.clear();
            app.add_metric_value.clear();
            app.add_metric_unit.clear();
            return;
        }

        let kind = app.add_metric_kind;
        let unit = app.add_metric_unit.trim().to_owned();
        let value: f64 = app.add_metric_value.trim().parse().unwrap_or(0.0);

        app.apply_cmd(Command::AddMetricDef {
            id: None,
            name: name.clone(),
            kind,
            unit,
            api: None,
        });

        app.apply_cmd(Command::AddMetricEntry {
            object: object_id,
            dir,
            id: None,
            metric_def: name.clone(),
            name,
            value,
            required: 0.0,
            max: None,
            fact: 0.0,
        });

        app.add_metric_target = None;
        app.add_metric_name.clear();
        app.add_metric_kind = MetricKind::Number;
        app.add_metric_value.clear();
        app.add_metric_unit.clear();
    }
}

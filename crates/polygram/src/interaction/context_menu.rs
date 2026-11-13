use crate::app::PolygramApp;
use polygram_core::types::{Direction, NodeKind};
use polygram_core::{Command, MetricValue, Published};
use egui::{Context, Window};

/// Контекстное меню по правой кнопке на объекте или на пустом месте холста.
pub fn context_menu(
    app: &mut PolygramApp,
    ctx: &Context,
    published: &Published,
    close_menu: &mut bool,
) {
    let menu_data = app.right_click.clone();
    if let Some((pos, object_id)) = &menu_data {
        Window::new("Контекстное меню")
            .default_pos(*pos)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                if let Some(object_id) = object_id {
                    if ui.button(format!("Удалить объект: {object_id}")).clicked() {
                        app.dialogs.delete_confirm = Some(vec![object_id.clone()]);
                        app.add_event_log(
                            "warn",
                            "dialog",
                            &format!("Запрошено удаление объекта «{object_id}» (подтверждение)"),
                            None,
                        );
                        *close_menu = true;
                    }
                    // Групповое удаление выделенных объектов (если выбрано несколько)
                    if app.selected_objects.len() > 1 {
                    if ui.button(format!("Удалить выделенные ({})", app.selected_objects.len())).clicked() {
                        app.dialogs.delete_confirm = Some(app.selected_objects.clone());
                        app.add_event_log(
                            "warn",
                            "dialog",
                            &format!("Запрошено удаление {} выделенных объектов (подтверждение)", app.selected_objects.len()),
                            None,
                        );
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
                            value: MetricValue::Null,
                            required: MetricValue::Null,
                            max: None,
                            fact: MetricValue::Null,
                        };
                        app.apply_cmd(cmd);
                        app.add_event_log(
                            "info",
                            "command",
                            &format!("Добавлен показатель «request{}» в объект «{object_id}» (запрос)", n_req + 1),
                            None,
                        );
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
                            value: MetricValue::Null,
                            required: MetricValue::Null,
                            max: None,
                            fact: MetricValue::Null,
                        };
                        app.apply_cmd(cmd);
                        app.add_event_log(
                            "info",
                            "command",
                            &format!("Добавлен показатель «out{}» в объект «{object_id}» (выход)", n_out + 1),
                            None,
                        );
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
                            app.add_event_log(
                                "info",
                                "command",
                                &format!("Создан объект «Новый процесс» в отделе «{object_id}» в позиции ({}, {})", world_pos.x.round() as i32, world_pos.y.round() as i32),
                                None,
                            );
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
                            app.add_event_log(
                                "info",
                                "command",
                                &format!("Создан отдел «Новый отдел» в отделе «{object_id}» в позиции ({}, {})", world_pos.x.round() as i32, world_pos.y.round() as i32),
                                None,
                            );
                            *close_menu = true;
                        }
                        // Переместить выделенные объекты в этот отдел.
                        if !app.selected_objects.is_empty()
                            && !app.selected_objects.iter().any(|s| s == object_id)
                        {
                            if ui.button(format!("Переместить выделенные ({}) в отдел", app.selected_objects.len())).clicked() {
                                let dept_id = object_id.clone();
                                let selected = app.selected_objects.clone();
                                let mut cmds: Vec<Command> = Vec::new();
                                for obj_id in &selected {
                                    cmds.push(Command::SetObjectParent {
                                        object: obj_id.clone(),
                                        parent: Some(dept_id.clone()),
                                    });
                                }
                                app.apply_batch(cmds);
                                app.add_event_log(
                                    "info",
                                    "command",
                                    &format!("{} выделенных объектов перемещены в отдел «{dept_id}»", selected.len()),
                                    None,
                                );
                                app.selected_objects.clear();
                                *close_menu = true;
                            }
                        }
                    ui.separator();
                }
                    // Извлечь объект из отдела (доступно только если объект вложен в отдел).
                    if let Some(o) = published
                        .geometry
                        .objects
                        .iter()
                        .find(|n| n.id == *object_id)
                    {
                        if o.parent.is_some() && o.kind != NodeKind::Department {
                            if ui.button("Извлечь из отдела").clicked() {
                                app.apply_cmd(Command::SetObjectParent {
                                    object: object_id.clone(),
                                    parent: None,
                                });
                                app.add_event_log(
                                    "info",
                                    "command",
                                    &format!("Объект «{object_id}» извлечён из отдела"),
                                    None,
                                );
                                *close_menu = true;
                            }
                        }
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
                        app.add_event_log(
                            "info",
                            "dialog",
                            &format!("Открыт диалог переименования объекта «{object_id}»"),
                            None,
                        );
                        *close_menu = true;
                    }
                }
                // Сгруппировать выделенные объекты в отдел (доступно при >=2 выделенных,
                // независимо от цели правого клика).
                if app.selected_objects.len() > 1 {
                    if ui.button("Сгруппировать в отдел").clicked() {
                        app.group_selected_into_department(&published);
                        app.add_event_log(
                            "info",
                            "command",
                            &format!("{} выделенных объектов сгруппированы в отдел", app.selected_objects.len()),
                            None,
                        );
                        *close_menu = true;
                    }
                    ui.separator();
                }
                if ui.button("Автораскладка").clicked() {
                    app.apply_cmd(Command::AutoLayout);
                    app.add_event_log("info", "command", "Запущена автораскладка графа", None);
                    *close_menu = true;
                }
                if ui.button("Перемаршрутизировать").clicked() {
                    app.apply_cmd(Command::RerouteAll);
                    app.add_event_log("info", "command", "Запущена перемаршрутизация связей", None);
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
                        app.add_event_log(
                            "info",
                            "command",
                            &format!("Создан процесс «Процесс» в позиции ({}, {})", world_pos.x.round() as i32, world_pos.y.round() as i32),
                            None,
                        );
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
                        app.add_event_log(
                            "info",
                            "command",
                            &format!("Создан отдел «Новый отдел» в позиции ({}, {})", world_pos.x as i32, world_pos.y as i32),
                            None,
                        );
                        *close_menu = true;
                    }
                }
            });
    }
    if *close_menu {
        app.right_click = None;
    }
}

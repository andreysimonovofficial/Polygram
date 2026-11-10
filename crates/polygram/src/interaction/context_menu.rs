use crate::app::PolygramApp;
use polygram_core::types::{Direction, NodeKind};
use polygram_core::{Command, Published};
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

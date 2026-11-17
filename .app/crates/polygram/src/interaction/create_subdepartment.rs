use crate::app::PolygramApp;
use polygram_core::{Command, NodeKind};
use egui::{Context, Window};

/// Модальное окно создания подотдела внутри выбранного отдела.
/// Пользователь вводит название, нажимает "Создать" — отдел добавляется
/// как ребёнок целевого отдела (SetObjectParent через AddObject с parent).
pub fn handle_create_subdepartment(app: &mut PolygramApp, ctx: &Context) {
    let Some(parent_id) = app.dialogs.create_subdepartment_target.clone() else {
        return;
    };

    let mut do_create = false;

    Window::new("Создать подотдел")
        .default_pos(egui::pos2(400.0, 300.0))
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label(format!("В отделе «{parent_id}»"));
            ui.label("Название:");
            ui.text_edit_singleline(&mut app.dialogs.create_subdepartment_name);
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Создать").clicked() {
                    do_create = true;
                }
                if ui.button("Отмена").clicked() {
                    app.dialogs.create_subdepartment_target = None;
                    app.dialogs.create_subdepartment_name.clear();
                }
            });
        });

    if do_create {
        let name = app.dialogs.create_subdepartment_name.trim().to_owned();
        if name.is_empty() {
            app.dialogs.create_subdepartment_target = None;
            app.dialogs.create_subdepartment_name.clear();
            return;
        }
        app.apply_batch(vec![
            Command::AddObject {
                id: None,
                name: name.clone(),
                kind: NodeKind::Department,
                at: None,
                parent: Some(parent_id.clone()),
                author: None,
            },
        ]);
        app.add_event_log(
            "info",
            "command",
            &format!("Создан подотдел «{name}» в отделе «{parent_id}»"),
            None,
        );
        app.dialogs.create_subdepartment_target = None;
        app.dialogs.create_subdepartment_name.clear();
    }
}

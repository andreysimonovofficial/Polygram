use crate::app::PolygramApp;
use egui::{Context, Window};

/// Подтверждение удаления объектов ("Удалить?").
/// Показывает диалог со списком объектов на удаление. При подтверждении
/// отправляет команды RemoveObject для каждого объекта и очищает выделение.
pub fn handle_delete_confirm(app: &mut PolygramApp, ctx: &Context) {
    let targets = match app.dialogs.delete_confirm.clone() {
        Some(t) if !t.is_empty() => t,
        _ => return,
    };

    let mut do_delete = false;
    let mut do_cancel = false;

    let count = targets.len();
    Window::new("Удалить?")
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
            app.apply_cmd(polygram_core::Command::RemoveObject { object: id.clone() });
        }
        app.add_event_log(
            "warn",
            "command",
            &format!("Удалены объекты: {}", targets.join(", ")),
            None,
        );
        // Удаляем удалённые объекты из выделения.
        app.selected_objects.retain(|s| !targets.iter().any(|t| t == s));
        app.dialogs.delete_confirm = None;
    }
}

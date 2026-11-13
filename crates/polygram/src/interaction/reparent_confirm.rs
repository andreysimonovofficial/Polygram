use crate::app::PolygramApp;
use polygram_core::Command;
use egui::{Context, Window};

/// Модальное окно "Добавить процесс в отдел?" при дропе процесса на отдел.
/// Да — процесс добавляется в список дочерних отдела (SetObjectParent) и
/// переезжает на целевую клетку (MoveObject); рамка отдела охватывает процесс.
/// Нет — отмена: процесс возвращается на прежние координаты, команды не отправляются.
pub fn handle_reparent_confirm(app: &mut PolygramApp, ctx: &Context) {
    let Some((object_id, dept_id, drop_cell, _start_cell)) =
        app.dialogs.reparent_confirm.clone()
    else {
        return;
    };

    let mut do_add = false;
    let mut do_cancel = false;

    Window::new("Добавить процесс в отдел?")
        .default_pos(egui::pos2(400.0, 300.0))
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label(format!("Добавить процесс «{object_id}» в отдел «{dept_id}»?"));
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Да").clicked() {
                    do_add = true;
                }
                if ui.button("Нет").clicked() {
                    do_cancel = true;
                }
            });
        });

    if do_cancel {
        // Возврат на прежние координаты: отменяем клиентское предсказание,
        // команды не отправляются — процесс остаётся на своём месте.
        app.drag.predicted_positions.remove(&object_id);
        app.dialogs.reparent_confirm = None;
        return;
    }

    if do_add {
        app.drag.pending_move =
            Some((object_id.clone(), app.connection.last_rev));
        app.apply_batch(vec![
            Command::SetObjectParent {
                object: object_id.clone(),
                parent: Some(dept_id.clone()),
            },
            Command::MoveObject {
                object: object_id.clone(),
                at: [drop_cell.0, drop_cell.1],
            },
        ]);
        app.selected_objects = vec![object_id.clone()];
        app.dialogs.reparent_confirm = None;
    }
}

use crate::app::PolygramApp;
use polygram_core::{Command, NodeKind, Published};
use egui::{Context, Window};
use polygram_core::layout::Rect as CoreRect;

/// Модальное окно подтверждения переноса объекта в отдел.
/// Для процесса: "Добавить процесс «X» в отдел «Y»?"
/// Для отдела: "Сделать отдел «X» подотделом отдела «Y»?"
/// Да — объект добавляется в список дочерних отдела (SetObjectParent) и
/// переезжает на целевую клетку (MoveObject); рамка отдела охватывает объект.
/// Нет — отмена: объект возвращается на прежние координаты, команды не отправляются.
pub fn handle_reparent_confirm(app: &mut PolygramApp, ctx: &Context, published: &Published) {
    let Some((object_id, dept_id, drop_cell, _start_cell, kind)) =
        app.dialogs.reparent_confirm.clone()
    else {
        return;
    };

    let mut do_add = false;
    let mut do_cancel = false;

    let title = match kind {
        NodeKind::Department => format!("Сделать отдел «{object_id}» подотделом?"),
        NodeKind::Process => format!("Добавить процесс в отдел?"),
        _ => format!("Перенести объект в отдел?"),
    };
    let body = match kind {
        NodeKind::Department => format!("Сделать отдел «{object_id}» подотделом отдела «{dept_id}»?"),
        NodeKind::Process => format!("Добавить процесс «{object_id}» в отдел «{dept_id}»?"),
        _ => format!("Перенести объект «{object_id}» в отдел «{dept_id}»?"),
    };

    Window::new(&title)
        .default_pos(egui::pos2(400.0, 300.0))
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label(&body);
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
        // команды не отправляются — объект остаётся на своём месте.
        app.drag.predicted_positions.remove(&object_id);
        app.add_event_log(
            "warn",
            "drag",
            &format!("Перенос объекта «{object_id}» в отдел «{dept_id}» отменён пользователем"),
            None,
        );
        app.dialogs.reparent_confirm = None;
        return;
    }

    if do_add {
        app.drag.pending_move =
            Some((object_id.clone(), app.connection.last_rev));

        // Проверить, не накрывает ли drop_cell другой объект в целевом отделе.
        // Если накрывает — искать свободную клетку рядом (как при призрачной копии).
        let mut final_cell = drop_cell;
        let object_size = published
            .geometry
            .objects
            .iter()
            .find(|o| o.id == object_id)
            .map(|o| (o.rect.w, o.rect.h))
            .unwrap_or((1, 1));
        let drop_rect = CoreRect::new(final_cell.0, final_cell.1, object_size.0, object_size.1);
        let overlaps = published.geometry.objects.iter().any(|o| {
            if o.id == object_id {
                return false;
            }
            if o.id == dept_id {
                return false;
            }
            o.rect.overlaps(&drop_rect)
        });
        if overlaps {
            // Поиск свободной клетки рядом с drop_cell внутри целевого отдела.
            let mut found = None;
            for r in 1..=12i32 {
                for dx in -r..=r {
                    for dy in -r..=r {
                        if dx.abs() != r && dy.abs() != r {
                            continue;
                        }
                        let cx = final_cell.0 + dx;
                        let cy = final_cell.1 + dy;
                        let r_test = CoreRect::new(cx, cy, object_size.0, object_size.1);
                        let free = !published.geometry.objects.iter().any(|o| {
                            if o.id == object_id {
                                return false;
                            }
                            if o.id == dept_id {
                                return false;
                            }
                            o.rect.overlaps(&r_test)
                        });
                        if free {
                            found = Some((cx, cy));
                            break;
                        }
                    }
                    if found.is_some() {
                        break;
                    }
                }
                if found.is_some() {
                    break;
                }
            }
            if let Some(fc) = found {
                final_cell = fc;
            } else {
                // Нет свободного места — отменяем перенос.
                app.drag.predicted_positions.remove(&object_id);
                app.drag.pending_move = None;
                app.add_event_log(
                    "warn",
                    "drag",
                    &format!("Перенос объекта «{object_id}» в отдел «{dept_id}» отменён: нет свободного места"),
                    None,
                );
                app.dialogs.reparent_confirm = None;
                return;
            }
        }

        let dx = final_cell.0 - _start_cell.0;
        let dy = final_cell.1 - _start_cell.1;
        let mut cmds: Vec<Command> = vec![
            Command::SetObjectParent {
                object: object_id.clone(),
                parent: Some(dept_id.clone()),
            },
            Command::MoveObject {
                object: object_id.clone(),
                at: [final_cell.0, final_cell.1],
            },
        ];
        // Отдел перевозит всех потомков с собой.
        if kind == NodeKind::Department {
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
        let kind_label = match kind {
            NodeKind::Department => "отдел",
            NodeKind::Process => "процесс",
            _ => "объект",
        };
        app.add_event_log(
            "info",
            "drag",
            &format!("{kind_label} «{object_id}» добавлен в отдел «{dept_id}» и перемещён на клетку ({}, {})", drop_cell.0, drop_cell.1),
            None,
        );
        app.selected_objects = vec![object_id.clone()];
        app.dialogs.reparent_confirm = None;
    }
}

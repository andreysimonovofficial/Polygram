//! Модальное окно подтверждения переноса объекта в отдел.
//! Для процесса: "Добавить процесс «X» в отдел «Y»?"

use crate::app::PolygramApp;
use polygram_core::{Command, NodeKind, Published};
use polygram_core::layout::Rect as CoreRect;
use egui::{Context, Window};

/// Модальное окно подтверждения переноса объекта в отдел.
/// Да — объект добавляется в список дочерних отдела (SetObjectParent) и
/// переезжает на целевую клетку (MoveObject); рамка отдела охватывает объект.
/// Нет — отмена: объект возвращается на прежние координаты.
pub fn handle_reparent_confirm(app: &mut PolygramApp, ctx: &Context, published: &Published) {
    let Some((object_id, dept_id, drop_cell, start_cell, kind)) =
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
        if let Some(o) = app.canvas.objects.iter_mut().find(|o| o.id == object_id) {
            o.rect.x = start_cell.0;
            o.rect.y = start_cell.1;
        }
        app.dialogs.reparent_confirm = None;
        return;
    }

    if do_add {
        let mut final_cell = drop_cell;
        let object_size = published
            .geometry
            .objects
            .iter()
            .find(|o| o.id == object_id)
            .map(|o| (o.rect.w, o.rect.h))
            .unwrap_or((16, 16));
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
            let dept_rect = published
                .geometry
                .objects
                .iter()
                .find(|o| o.id == dept_id && o.kind == NodeKind::Department)
                .map(|o| o.rect);
            let mut found = None;
            for r in 1..=12i32 {
                for dx in -r..=r {
                    for dy in -r..=r {
                        if dx.abs() != r && dy.abs() != r {
                            continue;
                        }
                        let cx = final_cell.0 + dx;
                        let cy = final_cell.1 + dy;
                        // Клетка должна быть внутри внутренней области отдела.
                        if let Some(dr) = dept_rect {
                            if cx < dr.x + 1
                                || cy < dr.y + 2
                                || cx + object_size.0 > dr.x + dr.w - 1
                                || cy + object_size.1 > dr.y + dr.h - 1
                            {
                                continue;
                            }
                        }
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
                // Нет свободного места — отменяем перенос, возвращаем на место.
                if let Some(o) = app.canvas.objects.iter_mut().find(|o| o.id == object_id) {
                    o.rect.x = start_cell.0;
                    o.rect.y = start_cell.1;
                }
                app.dialogs.reparent_confirm = None;
                return;
            }
        }

        let dx = final_cell.0 - start_cell.0;
        let dy = final_cell.1 - start_cell.1;
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
            let children: Vec<String> = published
                .geometry
                .objects
                .iter()
                .filter(|o| o.parent.as_deref() == Some(&object_id))
                .map(|o| o.id.clone())
                .collect();
            for child_id in children {
                if let Some(child) = published
                    .geometry
                    .objects
                    .iter()
                    .find(|o| o.id == child_id)
                {
                    cmds.push(Command::MoveObject {
                        object: child_id.clone(),
                        at: [child.rect.x + dx, child.rect.y + dy],
                    });
                }
            }
        }
        app.apply_batch(cmds);
        app.dialogs.reparent_confirm = None;
    }
}

use crate::engine::Engine;
use crate::{command::Applied, command::Command, error::Result};

mod auto_layout;
mod edges;
mod layout;
mod metric_defs;
mod metric_entries;
mod objects;

pub use auto_layout::handle_auto_layout;
pub use edges::{handle_connect, handle_disconnect};
pub use layout::handle_move_object;
pub use metric_defs::{handle_add_metric_def, handle_remove_metric_def, handle_rename_metric_def, handle_set_metric_def_unit};
pub use metric_entries::{
    handle_add_metric_entry, handle_remove_metric_entry, handle_reorder_metric_entry,
    handle_set_fact, handle_set_in_process, handle_set_max, handle_set_required, handle_set_value,
};
pub use objects::{handle_add_object, handle_remove_object, handle_rename_object, handle_set_object_parent};

pub fn exec(engine: &mut Engine, cmd: Command) -> Result<crate::command::Applied> {
    match cmd {
        Command::AddObject {
            id,
            name,
            kind,
            at,
            parent,
            author,
        } => handle_add_object(engine, id, name, kind, at, parent, author),
        Command::RemoveObject { object } => handle_remove_object(engine, &object),
        Command::RenameObject { object, name } => handle_rename_object(engine, &object, name),
        Command::SetObjectParent { object, parent } => {
            handle_set_object_parent(engine, &object, parent)
        }
        Command::MoveObject { object, at } => handle_move_object(engine, &object, at),
        Command::AddMetricDef {
            id,
            name,
            kind,
            unit,
            api,
        } => handle_add_metric_def(engine, id, name, kind, unit, api),
        Command::RemoveMetricDef { metric_def } => handle_remove_metric_def(engine, &metric_def),
        Command::RenameMetricDef { metric_def, name } => {
            handle_rename_metric_def(engine, &metric_def, name)
        }
        Command::SetMetricDefUnit { metric_def, unit } => {
            handle_set_metric_def_unit(engine, &metric_def, unit)
        }
        Command::AddMetricEntry {
            object,
            dir,
            id,
            metric_def,
            name: _,
            value,
            required,
            max,
            fact,
        } => handle_add_metric_entry(
            engine,
            &object,
            dir,
            id,
            &metric_def,
            value,
            required,
            max,
            fact,
        ),
        Command::RemoveMetricEntry { object, entry } => {
            handle_remove_metric_entry(engine, &object, &entry)
        }
        Command::ReorderMetricEntry {
            object,
            dir,
            from_ix,
            to_ix,
        } => handle_reorder_metric_entry(engine, &object, dir, from_ix, to_ix),
        Command::SetValue {
            object,
            entry,
            value,
        } => handle_set_value(engine, &object, &entry, value),
        Command::SetRequired {
            object,
            entry,
            required,
        } => handle_set_required(engine, &object, &entry, required),
        Command::SetMax { object, entry, max } => handle_set_max(engine, &object, &entry, max),
        Command::SetFact {
            object,
            entry,
            fact,
        } => handle_set_fact(engine, &object, &entry, fact),
        Command::SetInProcess {
            object,
            entry,
            in_process,
        } => handle_set_in_process(engine, &object, &entry, in_process),
        Command::Connect {
            from_object,
            from_entry,
            to_object,
            to_entry,
            throughput,
        } => handle_connect(
            engine,
            &from_object,
            &from_entry,
            &to_object,
            &to_entry,
            throughput,
        ),
        Command::Disconnect {
            from_object,
            from_entry,
            to_object,
            to_entry,
        } => handle_disconnect(engine, &from_object, &from_entry, &to_object, &to_entry),
        Command::AutoLayout => handle_auto_layout(engine),

        // FitCamera устанавливает флаг во вью камеры, чтобы GUI вписал все объекты в область просмотра.
        Command::FitCamera => {
            engine.camera_fit_requested = true;
            engine.rev += 1;
            Ok(Applied {
                rev: engine.rev,
                created: None,
                at: None,
            })
        }

        // AutoScreenshot включает/выключает автоматический снимок экрана при изменении состояния.
        Command::AutoScreenshot { enabled } => {
            engine.auto_screenshot = enabled;
            engine.rev += 1;
            Ok(Applied {
                rev: engine.rev,
                created: None,
                at: None,
            })
        }

        // Виртуальная мышь: перемещение курсора на экранные координаты.
        Command::VirtualMouseMove { x, y } => {
            engine.virtual_input.target_x = x as f32;
            engine.virtual_input.target_y = y as f32;
            engine.rev += 1;
            Ok(Applied {
                rev: engine.rev,
                created: None,
                at: None,
            })
        }

        // Виртуальная мышь: нажатие кнопки (0 — левая, 1 — правая).
        Command::VirtualMouseDown { button } => {
            match button {
                0 => engine.virtual_input.mouse_left_pressed = true,
                1 => engine.virtual_input.mouse_right_pressed = true,
                _ => {}
            }
            engine.rev += 1;
            Ok(Applied {
                rev: engine.rev,
                created: None,
                at: None,
            })
        }

        // Виртуальная мышь: отпускание кнопки (0 — левая, 1 — правая).
        Command::VirtualMouseUp { button } => {
            match button {
                0 => {
                    engine.virtual_input.mouse_left_pressed = false;
                    engine.virtual_input.target_x = 0.0;
                    engine.virtual_input.target_y = 0.0;
                }
                1 => {
                    engine.virtual_input.mouse_right_pressed = false;
                    engine.virtual_input.target_x = 0.0;
                    engine.virtual_input.target_y = 0.0;
                }
                _ => {}
            }
            engine.rev += 1;
            Ok(Applied {
                rev: engine.rev,
                created: None,
                at: None,
            })
        }

        // Виртуальная клавиатура: ввод текста.
        Command::VirtualKeyboardType { text } => {
            engine.virtual_input.keyboard_text = text;
            engine.rev += 1;
            Ok(Applied {
                rev: engine.rev,
                created: None,
                at: None,
            })
        }

        // Виртуальное рисование: установить фигуру для отрисовки от текущей позиции курсора до [x2, y2].
        // kind: 0 — эллипс, 1 — прямоугольник.
        Command::VirtualDrawShape { kind, x2, y2 } => {
            engine.virtual_input.draw_shape = Some((kind, x2 as f32, y2 as f32));
            engine.rev += 1;
            Ok(Applied {
                rev: engine.rev,
                created: None,
                at: None,
            })
        }

        // Виртуальная мышь: кривая Безье — движение к [x, y] через контрольную точку [cx, cy].
        Command::VirtualMouseCurve { x, y, cx, cy } => {
            engine.virtual_input.target_x = x as f32;
            engine.virtual_input.target_y = y as f32;
            engine.virtual_input.curve_control_x = cx as f32;
            engine.virtual_input.curve_control_y = cy as f32;
            engine.rev += 1;
            Ok(Applied {
                rev: engine.rev,
                created: None,
                at: None,
            })
        }

        // Виртуальная мышь: движение по скруглённой кривой через массив точек [(x,y), ...].
        Command::VirtualMousePath { points } => {
            engine.virtual_input.path_points = points.iter().map(|(x, y)| (*x as f32, *y as f32)).collect();
            engine.virtual_input.path_index = 0;
            engine.rev += 1;
            Ok(Applied {
                rev: engine.rev,
                created: None,
                at: None,
            })
        }

        // Админ-команды не модифицируют ядро, они управляют жизненным циклом API-сервера.
        Command::RestartServer => Ok(Applied { rev: 0, created: None, at: None }),
        Command::ChangePort { .. } => Ok(Applied { rev: 0, created: None, at: None }),
        Command::Shutdown => Ok(Applied { rev: 0, created: None, at: None }),
        Command::ReloadConfig => Ok(Applied { rev: 0, created: None, at: None }),
    }
}

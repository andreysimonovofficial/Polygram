use crate::app::PolygramApp;
use polygram_core::Published;
use egui::{Context, Window};

/// Диалог редактирования списка показателей контейнера (запросы/выходы) и
/// привязки к API.
pub fn handle_container_menu_dialog(
    app: &mut PolygramApp,
    ctx: &Context,
    published: &Published,
) {
    let Some((object_id, dir)) = &app.dialogs.container_menu_target else {
        return;
    };
    let object_id = object_id.clone();
    let dir = *dir;

    let label = match dir {
        polygram_core::types::Direction::Request => "ЗАПРОС",
        polygram_core::types::Direction::Output => "ВЫХОД",
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
        polygram_core::types::Direction::Request => obj.requests.iter().collect(),
        polygram_core::types::Direction::Output => obj.outputs.iter().collect(),
    };

    // Загрузить существующие API-привязки и заполнить поля диалога для первой записи.
    if app.dialogs.container_menu_api_connector.is_empty()
        && app.dialogs.container_menu_api_cell.is_empty()
    {
        let bindings = app.get_bindings();
        if let Some(first_entry) = entries.first() {
            let binding_id = format!("{}:{}", object_id, first_entry.name);
            for b in &bindings {
                if b.get("id").and_then(|v| v.as_str()) == Some(&binding_id) {
                    app.dialogs.container_menu_api_connector =
                        b.get("connector").and_then(|v| v.as_str()).unwrap_or("").to_owned();
                    app.dialogs.container_menu_api_cell =
                        b.get("cell").and_then(|v| v.as_str()).unwrap_or("").to_owned();
                    app.dialogs.container_menu_api_interval =
                        b.get("interval_secs").and_then(|v| v.as_u64()).unwrap_or(30);
                    break;
                }
            }
        }
    }

    // Инициализировать буферы редактирования (имя, значение, единица).
    // Для запроса редактируем required, для выхода — value.
    if app.dialogs.container_menu_edits.is_empty() {
        for entry in &entries {
            let buf_value = match dir {
                polygram_core::types::Direction::Request => format!(
                    "{:.0}",
                    published.dynamic.required[entry.ix as usize]
                ),
                polygram_core::types::Direction::Output => format!("{:.0}", entry.value),
            };
            app.dialogs.container_menu_edits.push((
                entry.name.clone(),
                buf_value,
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
                let buf_value = match dir {
                    polygram_core::types::Direction::Request => format!(
                        "{:.0}",
                        published.dynamic.required[entry.ix as usize]
                    ),
                    polygram_core::types::Direction::Output => format!("{:.0}", entry.value),
                };
                app.dialogs.container_menu_edits.push((
                    entry.name.clone(),
                    buf_value,
                    entry.unit.clone(),
                ));
            }
        }
    }

    let mut delete_entry: Option<String> = None;
    let mut apply_edits = false;
    let mut apply_api_binding = false;
    let mut close_dialog = false;

    Window::new(format!("Показатели — {label} ({object_id})"))
        .default_size([500.0, 450.0])
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
                        let field_label = match dir {
                            polygram_core::types::Direction::Request => "необходимое (required):",
                            polygram_core::types::Direction::Output => "значение (value):",
                        };
                        ui.label(field_label);

                        let buf_idx = app
                            .dialogs.container_menu_edits
                            .iter()
                            .position(|(n, _, _)| *n == entry.name);
                        if buf_idx.is_none() {
                            app.dialogs.container_menu_edits.push((
                                entry.name.clone(),
                                match dir {
                                    polygram_core::types::Direction::Request => format!(
                                        "{:.0}",
                                        published.dynamic.required[entry.ix as usize]
                                    ),
                                    polygram_core::types::Direction::Output => format!("{:.0}", entry.value),
                                },
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

                ui.separator();
                ui.heading("Синхронизация с API");
                ui.separator();

                let direction_text = match dir {
                    polygram_core::types::Direction::Request => "запись в: необходимое (required)",
                    polygram_core::types::Direction::Output => "запись в: значение (value)",
                };
                ui.label(direction_text);

                let connectors = app.get_connectors();
                let current_connector = &mut app.dialogs.container_menu_api_connector;
                egui::ComboBox::new("api_connector", "")
                    .selected_text(if current_connector.is_empty() {
                        "коннектор".to_owned()
                    } else {
                        current_connector.clone()
                    })
                    .show_ui(ui, |ui| {
                        if connectors.is_empty() {
                            ui.disable();
                            ui.label("нет коннекторов (API недоступен)");
                        } else {
                            for c in &connectors {
                                if ui.selectable_label(current_connector == c, c).clicked() {
                                    *current_connector = c.clone();
                                }
                            }
                        }
                    });

                ui.label("ячейка / метрика:");
                ui.text_edit_singleline(&mut app.dialogs.container_menu_api_cell);

                ui.label("интервал опроса (сек):");
                let mut interval_str = app.dialogs.container_menu_api_interval.to_string();
                if ui.text_edit_singleline(&mut interval_str).lost_focus() {
                    if let Ok(interval) = interval_str.trim().parse::<u64>() {
                        app.dialogs.container_menu_api_interval = interval.max(1);
                    }
                }

                if !app.dialogs.container_menu_api_connector.is_empty()
                    && !app.dialogs.container_menu_api_cell.is_empty()
                {
                    ui.label(format!(
                        "привязка: {} → {}",
                        app.dialogs.container_menu_api_connector,
                        app.dialogs.container_menu_api_cell
                    ));
                }
            });

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Применить").clicked() {
                    apply_edits = true;
                }
                if !app.dialogs.container_menu_api_connector.is_empty()
                    && !app.dialogs.container_menu_api_cell.is_empty()
                {
                    if ui.button("Привязать к API").clicked() {
                        apply_api_binding = true;
                    }
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
            match dir {
                polygram_core::types::Direction::Request => app.apply_cmd(polygram_core::Command::SetRequired {
                    object: object_id.clone(),
                    entry: entry_name,
                    required: value,
                }),
                polygram_core::types::Direction::Output => app.apply_cmd(polygram_core::Command::SetValue {
                    object: object_id.clone(),
                    entry: entry_name,
                    value,
                }),
            }
        }
        for (metric_def, unit) in unit_cmds {
            app.apply_cmd(polygram_core::Command::SetMetricDefUnit {
                metric_def,
                unit,
            });
        }
    }

    // Применить API-привязку.
    if apply_api_binding {
        let connector = app.dialogs.container_menu_api_connector.clone();
        let cell = app.dialogs.container_menu_api_cell.clone();
        let interval = app.dialogs.container_menu_api_interval;
        // Привязываем к первой записи в списке (пользователь может повторить для остальных).
        if let Some(first_entry) = entries.first() {
            app.post_binding(
                object_id.clone(),
                first_entry.name.clone(),
                connector,
                cell,
                interval,
            );
        }
        app.dialogs.container_menu_api_connector.clear();
        app.dialogs.container_menu_api_cell.clear();
        app.dialogs.container_menu_api_interval = 30;
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
        app.dialogs.container_menu_api_connector.clear();
        app.dialogs.container_menu_api_cell.clear();
        app.dialogs.container_menu_api_interval = 30;
    }
}

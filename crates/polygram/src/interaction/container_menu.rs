//! Диалог редактирования списка показателей контейнера (запросы/выходы) и
//! привязки к API.

use crate::app::{PolygramApp, dialogs::EntryApiBinding};
use crate::interaction::api_settings::render_entry_api_settings;
use polygram_core::{MetricValue, Published};
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

    // Загрузить существующие API-привязки для каждой записи в настройки диалога.
    if app.dialogs.container_menu_api_settings.is_empty() {
        let bindings = app.get_bindings();
        for entry in &entries {
            let binding_id = format!("{}:{}", object_id, entry.name);
            for b in &bindings {
                if b.get("id").and_then(|v| v.as_str()) == Some(&binding_id) {
                    app.dialogs.container_menu_api_settings.insert(
                        entry.name.clone(),
                        EntryApiBinding {
                            connector: b.get("connector").and_then(|v| v.as_str()).unwrap_or("").to_owned(),
                            cell: b.get("cell").and_then(|v| v.as_str()).unwrap_or("").to_owned(),
                            interval_secs: b.get("interval_secs").and_then(|v| v.as_u64()).unwrap_or(30),
                        },
                    );
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

                        // Статус привязки и время последнего опроса для этой записи.
                        let binding_id = format!("{}:{}", object_id, entry.name);
                        if let Some(info) = app.bindings.api_bindings.get(&binding_id) {
                            let icon_color = if info.ok {
                                egui::Color32::from_rgb(0, 200, 80)
                            } else {
                                egui::Color32::from_rgb(240, 80, 80)
                            };
                            let icon_text = if info.ok { "🌐" } else { "⚠" };
                            // Кнопка иконки привязки: при клике на ⚠ копируем текст ошибки.
                            let btn = egui::Button::new(icon_text);
                            if ui.add(btn).clicked() {
                                if let Some(e) = &info.error {
                                    ctx.copy_text(e.clone());
                                }
                            }
                            if let Some(ts) = &info.last_updated {
                                ui.label(ts);
                            }
                        }

                        if ui.button("⚙").clicked() {
                            if app
                                .dialogs
                                .container_menu_api_expanded
                                .as_deref()
                                == Some(entry.name.as_str())
                            {
                                app.dialogs.container_menu_api_expanded = None;
                            } else {
                                app.dialogs.container_menu_api_expanded =
                                    Some(entry.name.clone());
                            }
                        }
                        if ui.button("🗑").clicked() {
                            delete_entry = Some(entry.name.clone());
                        }
                    });

                    // Развёрнутые настройки привязки для этой записи.
                    if app
                        .dialogs
                        .container_menu_api_expanded
                        .as_deref()
                        == Some(entry.name.as_str())
                    {
                        render_entry_api_settings(ui, app, &object_id, *entry, dir);
                    }
                }
            });

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Применить").clicked() {
                    apply_edits = true;
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
                    entry: entry_name.clone(),
                    required: MetricValue::Number(value),
                }),
                polygram_core::types::Direction::Output => app.apply_cmd(polygram_core::Command::SetValue {
                    object: object_id.clone(),
                    entry: entry_name.clone(),
                    value: MetricValue::Number(value),
                }),
            }
            app.add_event_log(
                "info",
                "command",
                &format!("Объект «{object_id}»: запись «{entry_name}» установлена в {value}"),
                None,
            );
        }
        for (metric_def, unit) in unit_cmds {
            app.apply_cmd(polygram_core::Command::SetMetricDefUnit {
                metric_def: metric_def.clone(),
                unit: unit.clone(),
            });
            app.add_event_log(
                "info",
                "command",
                &format!("Показатель «{metric_def}»: единица измерения изменена на «{unit}»"),
                None,
            );
        }
    }

    // Удалить показатель.
    if let Some(entry_name) = delete_entry {
        app.apply_cmd(polygram_core::Command::RemoveMetricEntry {
            object: object_id.clone(),
            entry: entry_name.clone(),
        });
        app.add_event_log(
            "warn",
            "command",
            &format!("Объект «{object_id}»: удалена запись «{entry_name}»"),
            None,
        );
    }

    if close_dialog {
        app.dialogs.container_menu_target = None;
        app.dialogs.container_menu_edits.clear();
        app.dialogs.container_menu_api_settings.clear();
        app.dialogs.container_menu_api_expanded = None;
    }
}

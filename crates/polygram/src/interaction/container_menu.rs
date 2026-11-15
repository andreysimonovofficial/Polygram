//! Диалог редактирования списка показателей объекта (запросы и выходы) в едином окне.
//! Два раскрывающихся раздела: ЗАПРОС и ВЫХОД с показаниями каждого.

use crate::app::{PolygramApp, dialogs::EntryApiBinding};
use crate::interaction::api_settings::render_entry_api_settings;
use polygram_core::{Direction, MetricValue, Published};
use polygram_canvas::Theme;
use egui::{Context, Window};

/// Диалог редактирования списка показателей объекта в едином окне с двумя разделами.
pub fn handle_container_menu_dialog(
    app: &mut PolygramApp,
    ctx: &Context,
    published: &Published,
    theme: &Theme,
) {
    let Some(object_id) = &app.dialogs.container_menu_target else {
        return;
    };
    let object_id = object_id.clone();
    let active_section = app.dialogs.container_menu_active_section;

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

    let requests: Vec<&polygram_core::geometry::EntryGeom> = obj.requests.iter().collect();
    let outputs: Vec<&polygram_core::geometry::EntryGeom> = obj.outputs.iter().collect();

    // Загрузить существующие API-привязки для всех записей обоих направлений.
    if app.dialogs.container_menu_api_settings.is_empty() {
        let bindings = app.get_bindings();
        for entry in requests.iter().chain(outputs.iter()) {
            let binding_id = format!("{}:{}", object_id, entry.name);
            for b in &bindings {
                if b.get("id").and_then(|v| v.as_str()) == Some(&binding_id) {
                    app.dialogs.container_menu_api_settings.insert(
                        entry.name.clone(),
                        EntryApiBinding {
                            connector: b
                                .get("connector")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_owned(),
                            cell: b.get("cell").and_then(|v| v.as_str()).unwrap_or("").to_owned(),
                            interval_secs: b
                                .get("interval_secs")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(30),
                        },
                    );
                    break;
                }
            }
        }
    }

    // Инициализировать буферы редактирования для обоих направлений.
    app.dialogs
        .ensure_container_menu_edits(&requests, Direction::Request, published);
    app.dialogs
        .ensure_container_menu_edits(&outputs, Direction::Output, published);

    let mut delete_entry: Option<(String, Direction)> = None;
    let mut apply_edits = false;
    let mut close_dialog = false;

    Window::new(format!("Показатели — {object_id}"))
        .default_size([500.0, 450.0])
        .resizable(true)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label(format!("Объект «{object_id}»"));
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                // Раздел: ЗАПРОС
                let req_is_open = app
                    .dialogs
                    .container_menu_section_open
                    .get(&Direction::Request)
                    .copied()
                    .unwrap_or(false);
                let req_arrow = if req_is_open { "▼" } else { "▶" };
                let req_header_resp = ui.horizontal(|ui| {
                    ui.label(req_arrow);
                    ui.label(format!("ЗАПРОС ({})", requests.len()));
                });
                if req_header_resp.response.clicked() {
                    // ВЫХОД сворачивается при открытии/переключении ЗАПРОС.
                    app.dialogs
                        .container_menu_section_open
                        .insert(Direction::Output, false);
                    app.dialogs
                        .container_menu_section_open
                        .insert(Direction::Request, !req_is_open);
                }
                if req_is_open {
                    render_metric_section(
                        ui,
                        app,
                        ctx,
                        published,
                        theme,
                        &object_id,
                        Direction::Request,
                        &requests,
                        &mut delete_entry,
                    );
                }

                ui.separator();

                // Раздел: ВЫХОД
                let out_is_open = app
                    .dialogs
                    .container_menu_section_open
                    .get(&Direction::Output)
                    .copied()
                    .unwrap_or(false);
                let out_arrow = if out_is_open { "▼" } else { "▶" };
                let out_header_resp = ui.horizontal(|ui| {
                    ui.label(out_arrow);
                    ui.label(format!("ВЫХОД ({})", outputs.len()));
                });
                if out_header_resp.response.clicked() {
                    // ЗАПРОС сворачивается при открытии/переключении ВЫХОД.
                    app.dialogs
                        .container_menu_section_open
                        .insert(Direction::Request, false);
                    app.dialogs
                        .container_menu_section_open
                        .insert(Direction::Output, !out_is_open);
                }
                if out_is_open {
                    render_metric_section(
                        ui,
                        app,
                        ctx,
                        published,
                        theme,
                        &object_id,
                        Direction::Output,
                        &outputs,
                        &mut delete_entry,
                    );
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
        let (value_cmds, unit_cmds) = app.dialogs.apply_container_edits();
        for (entry_name, dir, value) in value_cmds {
            match dir {
                Direction::Request => app.apply_cmd(polygram_core::Command::SetRequired {
                    object: object_id.clone(),
                    entry: entry_name.clone(),
                    required: MetricValue::Number(value),
                }),
                Direction::Output => app.apply_cmd(polygram_core::Command::SetValue {
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
    if let Some((entry_name, dir)) = delete_entry {
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
        app.dialogs.container_menu_active_section = None;
        app.dialogs.container_menu_edits.clear();
        app.dialogs.container_menu_api_settings.clear();
        app.dialogs.container_menu_api_expanded = None;
    }
}

/// Отрисовать список метрик одного направления внутри раздела.
fn render_metric_section(
    ui: &mut egui::Ui,
    app: &mut PolygramApp,
    ctx: &Context,
    published: &Published,
    theme: &Theme,
    object_id: &str,
    dir: Direction,
    entries: &[&polygram_core::geometry::EntryGeom],
    delete_entry: &mut Option<(String, Direction)>,
) {
    let field_label = match dir {
        Direction::Request => "необходимое (required):",
        Direction::Output => "значение (value):",
    };

    for entry in entries.iter() {
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(&entry.name);
            ui.label(field_label);

            let buf_idx = app.dialogs.find_container_menu_edit(&entry.name, dir);
            if buf_idx.is_none() {
                app.dialogs.container_menu_edits.push((
                    entry.name.clone(),
                    dir,
                    match dir {
                        Direction::Request => format!(
                            "{:.0}",
                            published.dynamic.required[entry.ix as usize]
                        ),
                        Direction::Output => format!("{:.0}", entry.value),
                    },
                    entry.unit.clone(),
                ));
            }
            let buf_idx = app
                .dialogs
                .find_container_menu_edit(&entry.name, dir)
                .unwrap();
            let (value_buf, unit_buf) = {
                let e = &mut app.dialogs.container_menu_edits[buf_idx];
                (&mut e.2, &mut e.3)
            };

            ui.text_edit_singleline(value_buf);
            ui.label("единица:");
            ui.text_edit_singleline(unit_buf);

            // Статус привязки и время последнего опроса для этой записи.
            let binding_id = format!("{}:{}", object_id, entry.name);
            if let Some(info) = app.bindings.api_bindings.get(&binding_id) {
                let icon_text = if info.ok { "🌐" } else { "⚠" };
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
                *delete_entry = Some((entry.name.clone(), dir));
            }
        });

        // Развёрнутые настройки привязки для этой записи.
        if app
            .dialogs
            .container_menu_api_expanded
            .as_deref()
            == Some(entry.name.as_str())
        {
            render_entry_api_settings(ui, app, object_id, *entry, dir);
        }
    }
}

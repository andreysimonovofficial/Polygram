//! Экран редактирования списка показателей объекта (запросы и результаты) — полноэкранный вид.
//! Два раскрывающихся раздела: ЗАПРОС и РЕЗУЛЬТАТ с показаниями каждого.

use crate::app::{PolygramApp, dialogs::EntryApiBinding, dialogs::ContainerMenuTab};
use crate::icons::lucide_icon;
use crate::interaction::api_settings::render_entry_api_settings;
use polygram_core::{Direction, MetricValue, Published};
use polygram_canvas::Theme;
use egui::Context;
use egui_lucide::Lucide;

/// Переключает приложение на полноэкранный вид показателей объекта.
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
    let results: Vec<&polygram_core::geometry::EntryGeom> = obj.results.iter().collect();

    // Загрузить существующие API-привязки для всех записей обоих направлений.
    if app.dialogs.container_menu_api_settings.is_empty() {
        let bindings = app.get_bindings();
        for entry in requests.iter().chain(results.iter()) {
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
        .ensure_container_menu_edits(&results, Direction::Result, published);

    // Переключаемся на полноэкранный вид показателей.
    app.show_metrics_screen = true;
    app.metrics_target_object = Some(object_id);
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
        Direction::Result => "значение (value):",
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
                        Direction::Result => format!("{:.0}", entry.value),
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
                let icon = if info.ok { Lucide::Globe } else { Lucide::TriangleAlert };
                let color = if info.ok { theme.status_ok } else { theme.status_deficit };
                if lucide_icon(ui, icon, 16.0, color).clicked() {
                    if let Some(e) = &info.error {
                        ctx.copy_text(e.clone());
                    }
                }
                if let Some(ts) = &info.last_updated {
                    ui.label(ts);
                }
            }

            if lucide_icon(ui, Lucide::Settings, 16.0, theme.text).clicked() {
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
            if lucide_icon(ui, Lucide::Trash, 16.0, theme.text).clicked() {
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

/// Рендерит полноэкранный вид показателей объекта (вместо CentralPanel с холстом).
pub fn render_metrics_screen(
    app: &mut PolygramApp,
    ctx: &Context,
    ui: &mut egui::Ui,
    published: &Published,
    theme: &Theme,
) -> bool {
    // Возвращает true, если нужно закрыть экран и вернуться к холсту.
    let mut close_screen = false;

    let object_id = match &app.metrics_target_object {
        Some(id) => id.clone(),
        None => return false,
    };

    let obj = match published
        .geometry
        .objects
        .iter()
        .find(|o| o.id == *object_id)
    {
        Some(o) => o,
        None => {
            app.show_metrics_screen = false;
            app.metrics_target_object = None;
            return false;
        }
    };

    let requests: Vec<&polygram_core::geometry::EntryGeom> = obj.requests.iter().collect();
    let results: Vec<&polygram_core::geometry::EntryGeom> = obj.results.iter().collect();

    // Заголовок с кнопкой возврата.
    ui.horizontal(|ui| {
        if lucide_icon(ui, Lucide::ArrowLeft, 14.0, theme.text).clicked() || ui.button("Назад").clicked() {
            close_screen = true;
        }
        ui.label(format!("Показатели — объект «{object_id}»"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(format!("({} запросов, {} результатов)", requests.len(), results.len()));
        });
    });
    ui.separator();

    // Вкладки: Метрики / Чат.
    egui::Frame::new().inner_margin(0.0).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing();
            let metrics_btn = egui::Button::new("Метрики")
                .fill(if app.dialogs.container_menu_tab == ContainerMenuTab::Metrics {
                    ui.style().visuals.weak_text_color().linear_multiply(0.1)
                } else {
                    egui::Color32::TRANSPARENT
                });
            if ui.add(metrics_btn).clicked() {
                app.dialogs.container_menu_tab = ContainerMenuTab::Metrics;
            }
            ui.separator();
            let chat_btn = egui::Button::new("Чат")
                .fill(if app.dialogs.container_menu_tab == ContainerMenuTab::Chat {
                    ui.style().visuals.weak_text_color().linear_multiply(0.1)
                } else {
                    egui::Color32::TRANSPARENT
                });
            if ui.add(chat_btn).clicked() {
                app.dialogs.container_menu_tab = ContainerMenuTab::Chat;
            }
            ui.spacing();
        });
    });
    ui.separator();

    match app.dialogs.container_menu_tab {
        ContainerMenuTab::Metrics => {
            // Инициализировать буферы редактирования.
            app.dialogs
                .ensure_container_menu_edits(&requests, Direction::Request, published);
            app.dialogs
                .ensure_container_menu_edits(&results, Direction::Result, published);

            let mut delete_entry: Option<(String, Direction)> = None;

            egui::ScrollArea::vertical().show(ui, |ui| {
                // Раздел: ЗАПРОС
                let req_is_open = app
                    .dialogs
                    .container_menu_section_open
                    .get(&Direction::Request)
                    .copied()
                    .unwrap_or(false);
                let req_arrow = if req_is_open { Lucide::ChevronDown } else { Lucide::ChevronRight };
                let req_header_resp = ui.horizontal(|ui| {
                    ui.add(req_arrow.size(14.0));
                    ui.label(format!("ЗАПРОС ({})", requests.len()));
                });
                if req_header_resp.response.clicked() {
                    app.dialogs
                        .container_menu_section_open
                        .insert(Direction::Result, false);
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

                // Раздел: РЕЗУЛЬТАТ
                let result_is_open = app
                    .dialogs
                    .container_menu_section_open
                    .get(&Direction::Result)
                    .copied()
                    .unwrap_or(false);
                let result_arrow = if result_is_open { Lucide::ChevronDown } else { Lucide::ChevronRight };
                let result_header_resp = ui.horizontal(|ui| {
                    ui.add(result_arrow.size(14.0));
                    ui.label(format!("РЕЗУЛЬТАТ ({})", results.len()));
                });
                if result_header_resp.response.clicked() {
                    app.dialogs
                        .container_menu_section_open
                        .insert(Direction::Request, false);
                    app.dialogs
                        .container_menu_section_open
                        .insert(Direction::Result, !result_is_open);
                }
                if result_is_open {
                    render_metric_section(
                        ui,
                        app,
                        ctx,
                        published,
                        theme,
                        &object_id,
                        Direction::Result,
                        &results,
                        &mut delete_entry,
                    );
                }
            });

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Применить").clicked() {
                    let (value_cmds, unit_cmds) = app.dialogs.apply_container_edits();
                    for (entry_name, dir, value) in value_cmds {
                        match dir {
                            Direction::Request => app.apply_cmd(polygram_core::Command::SetRequired {
                                object: object_id.clone(),
                                entry: entry_name.clone(),
                                required: MetricValue::Number(value),
                            }),
                            Direction::Result => app.apply_cmd(polygram_core::Command::SetValue {
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
                if ui.button("Закрыть").clicked() {
                    close_screen = true;
                }
            });

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
        }
        ContainerMenuTab::Chat => {
            ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::TopDown), |ui| {
                ui.heading("Чат");
                ui.label("Здесь будет чат объекта.");
            });
        }
    }

    if close_screen {
        app.show_metrics_screen = false;
        app.metrics_target_object = None;
        app.dialogs.container_menu_target = None;
        app.dialogs.container_menu_active_section = None;
        app.dialogs.container_menu_section_open.clear();
        app.dialogs.container_menu_edits.clear();
        app.dialogs.container_menu_api_settings.clear();
        app.dialogs.container_menu_api_expanded = None;
    }

    close_screen
}

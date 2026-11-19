//! Экран редактирования списка показателей объекта (запросы и результаты) — полноэкранный вид.
//! Два раскрывающихся раздела: ЗАПРОС и РЕЗУЛЬТАТ с показаниями каждого.

use crate::app::{PolygramApp, dialogs::ContainerMenuTab};
use polygram_core::{Direction, MetricValue, runtime::Published};
use polygram_canvas::Theme;
use egui_lucide::Lucide;

/// Переключает приложение на полноэкранный вид показателей объекта.
pub fn handle_container_menu_dialog(
    app: &mut PolygramApp,
    published: &Published,
    _theme: Theme,
) {
    let Some(object_id) = &app.dialogs.container_menu_target.clone() else {
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

    app.dialogs
        .ensure_container_menu_edits(&requests, Direction::Request, published);
    app.dialogs
        .ensure_container_menu_edits(&results, Direction::Result, published);

    app.dialogs.show_metrics_screen = true;
    app.dialogs.metrics_target_object = Some(object_id);
}

/// Отрисовать список метрик одного направления внутри раздела.
fn render_metric_section(
    ui: &mut egui::Ui,
    app: &mut PolygramApp,
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
            let buf_idx = app.dialogs.find_container_menu_edit(&entry.name, dir).unwrap();
            let (value_buf, unit_buf) = {
                let e = &mut app.dialogs.container_menu_edits[buf_idx];
                (&mut e.2, &mut e.3)
            };

            ui.text_edit_singleline(value_buf);
            ui.label("единица:");
            ui.text_edit_singleline(unit_buf);

            if ui
                .add(
                    egui::Button::image(
                        egui_lucide::Lucide::Trash
                            .color(theme.text)
                            .size(16.0)
                            .image(),
                    )
                    .frame(false)
                    .fill(egui::Color32::TRANSPARENT),
                )
                .clicked()
            {
                *delete_entry = Some((entry.name.clone(), dir));
            }
        });
    }
}

/// Рендерит полноэкранный вид показателей объекта (вместо CentralPanel с холстом).
pub fn render_metrics_screen(
    app: &mut PolygramApp,
    ui: &mut egui::Ui,
    published: &Published,
    theme: Theme,
) -> bool {
    let mut close_screen = false;

    let object_id = match &app.dialogs.metrics_target_object {
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
            app.dialogs.show_metrics_screen = false;
            app.dialogs.metrics_target_object = None;
            return false;
        }
    };

    let requests: Vec<&polygram_core::geometry::EntryGeom> = obj.requests.iter().collect();
    let results: Vec<&polygram_core::geometry::EntryGeom> = obj.results.iter().collect();

    ui.horizontal(|ui| {
        if ui
            .add(
                egui::Button::image(
                    egui_lucide::Lucide::ArrowLeft
                        .color(theme.text)
                        .size(14.0)
                        .image(),
                )
                .frame(false)
                .fill(egui::Color32::TRANSPARENT),
            )
            .clicked()
            || ui.button("Назад").clicked()
        {
            close_screen = true;
        }
        ui.label(format!("Показатели — объект «{object_id}»"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(format!("({} запросов, {} результатов)", requests.len(), results.len()));
        });
    });
    ui.separator();

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
            app.dialogs
                .ensure_container_menu_edits(&requests, Direction::Request, published);
            app.dialogs
                .ensure_container_menu_edits(&results, Direction::Result, published);

            let mut delete_entry: Option<(String, Direction)> = None;

            egui::ScrollArea::vertical().show(ui, |ui| {
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
                        published,
                        &theme,
                        &object_id,
                        Direction::Request,
                        &requests,
                        &mut delete_entry,
                    );
                }

                ui.separator();

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
                        published,
                        &theme,
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
                    }
                    for (metric_def, unit) in unit_cmds {
                        app.apply_cmd(polygram_core::Command::SetMetricDefUnit {
                            metric_def: metric_def.clone(),
                            unit: unit.clone(),
                        });
                    }
                }
                if ui.button("Закрыть").clicked() {
                    close_screen = true;
                }
            });

            if let Some((entry_name, dir)) = delete_entry {
                app.apply_cmd(polygram_core::Command::RemoveMetricEntry {
                    object: object_id.clone(),
                    entry: entry_name.clone(),
                });
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
        app.dialogs.show_metrics_screen = false;
        app.dialogs.metrics_target_object = None;
        app.dialogs.container_menu_target = None;
        app.dialogs.container_menu_active_section = None;
        app.dialogs.container_menu_section_open.clear();
        app.dialogs.container_menu_edits.clear();
        app.dialogs.container_menu_api_settings.clear();
        app.dialogs.container_menu_api_expanded = None;
    }

    close_screen
}

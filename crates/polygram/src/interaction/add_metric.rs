use crate::app::PolygramApp;
use polygram_core::{Command, MetricKind, Published};
use egui::{Context, Window};

/// Модальное диалоговое окно добавления показателя.
pub fn handle_add_metric_dialog(
    app: &mut PolygramApp,
    ctx: &Context,
    published: &Published,
) {
    let Some((object_id, dir)) = &app.dialogs.add_metric_target else {
        return;
    };
    let object_id = object_id.clone();
    let dir = *dir;

    let label = match dir {
        polygram_core::types::Direction::Request => "ЗАПРОС",
        polygram_core::types::Direction::Output => "ВЫХОД",
    };
    let value_label = match dir {
        polygram_core::types::Direction::Request => "Необходимое (required):",
        polygram_core::types::Direction::Output => "Значение (value):",
    };

    let mut done = false;
    let mut cancelled = false;

    // Ищем существующий показатель по введённому имени (для подстановки данных).
    let existing_md = published
        .metric_defs
        .iter()
        .find(|md| md.name == app.dialogs.add_metric_name.trim())
        .cloned();

    Window::new(format!("Добавить показатель — {label}"))
        .default_size([320.0, 240.0])
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label("Имя показателя:");
            ui.text_edit_singleline(&mut app.dialogs.add_metric_name);

            if let Some(md) = &existing_md {
                ui.label(format!("Найден существующий показатель: {}", md.name));
                ui.label(format!("Тип: {:?}", md.kind));
                ui.label(format!("Единица: {}", md.unit));
                ui.separator();
                ui.label(value_label);
                ui.text_edit_singleline(&mut app.dialogs.add_metric_value);
            } else {
                ui.separator();
                ui.label("Тип:");
                egui::ComboBox::new("add_metric_kind", "")
                    .selected_text(format!("{:?}", app.dialogs.add_metric_kind))
                    .show_ui(ui, |ui| {
                        for kind in [
                            MetricKind::Number,
                            MetricKind::Time,
                            MetricKind::Percentage,
                            MetricKind::Binary,
                            MetricKind::Duration,
                            MetricKind::Formula,
                        ] {
                            if ui.selectable_label(app.dialogs.add_metric_kind == kind, format!("{:?}", kind)).clicked() {
                                app.dialogs.add_metric_kind = kind;
                            }
                        }
                    });

                ui.separator();
                ui.label(value_label);
                ui.text_edit_singleline(&mut app.dialogs.add_metric_value);

                ui.separator();
                ui.label("Единица измерения:");
                ui.text_edit_singleline(&mut app.dialogs.add_metric_unit);
            }

            ui.separator();
            ui.heading("Синхронизация с API (необязательно)");
            let connectors = app.get_connectors();
            let current_connector = &mut app.dialogs.add_metric_api_connector;
            egui::ComboBox::new("add_api_connector", "")
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
            ui.text_edit_singleline(&mut app.dialogs.add_metric_api_cell);

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    done = true;
                }
                if ui.button("Отмена").clicked() {
                    cancelled = true;
                }
            });
        });

    if cancelled {
        app.dialogs.add_metric_target = None;
        app.dialogs.add_metric_name.clear();
        app.dialogs.add_metric_value.clear();
        app.dialogs.add_metric_unit.clear();
        app.dialogs.add_metric_api_connector.clear();
        app.dialogs.add_metric_api_cell.clear();
        return;
    }

    if done {
        let name = app.dialogs.add_metric_name.trim().to_owned();
        if name.is_empty() {
            app.dialogs.add_metric_target = None;
            app.dialogs.add_metric_name.clear();
            app.dialogs.add_metric_value.clear();
            app.dialogs.add_metric_unit.clear();
            return;
        }

        let value: f64 = app.dialogs.add_metric_value.trim().parse().unwrap_or(0.0);
        let object_id_for_cmd = object_id.clone();
        let name_for_cmd = name.clone();

        if let Some(md) = &existing_md {
            // Используем существующий показатель — создаём только запись на объекте.
            // Для запроса вводимое значение — необходимое (required), для выхода — значение (value).
            let (entry_value, entry_required) = match dir {
                polygram_core::types::Direction::Request => (0.0, value),
                polygram_core::types::Direction::Output => (value, 0.0),
            };
            app.apply_batch(vec![Command::AddMetricEntry {
                object: object_id_for_cmd.clone(),
                dir,
                id: Some(name_for_cmd.clone()),
                metric_def: md.name.clone(),
                name: name_for_cmd.clone(),
                value: entry_value,
                required: entry_required,
                max: None,
                fact: 0.0,
            }]);
        } else {
            let kind = app.dialogs.add_metric_kind;
            let unit = app.dialogs.add_metric_unit.trim().to_owned();
            let (entry_value, entry_required) = match dir {
                polygram_core::types::Direction::Request => (0.0, value),
                polygram_core::types::Direction::Output => (value, 0.0),
            };

            app.apply_batch(vec![
                Command::AddMetricDef {
                    id: Some(name_for_cmd.clone()),
                    name: name_for_cmd.clone(),
                    kind,
                    unit,
                    api: None,
                },
                Command::AddMetricEntry {
                    object: object_id_for_cmd.clone(),
                    dir,
                    id: Some(name_for_cmd.clone()),
                    metric_def: name_for_cmd.clone(),
                    name: name_for_cmd.clone(),
                    value: entry_value,
                    required: entry_required,
                    max: None,
                    fact: 0.0,
                },
            ]);

            // API-привязка нового показателя.
            if !app.dialogs.add_metric_api_connector.is_empty()
                && !app.dialogs.add_metric_api_cell.is_empty()
            {
                app.post_binding(
                    object_id_for_cmd.clone(),
                    name_for_cmd.clone(),
                    app.dialogs.add_metric_api_connector.clone(),
                    app.dialogs.add_metric_api_cell.clone(),
                    30,
                );
            }
        }

        // API-привязка к существующему показателю.
        if !app.dialogs.add_metric_api_connector.is_empty()
            && !app.dialogs.add_metric_api_cell.is_empty()
        {
            app.post_binding(
                object_id_for_cmd.clone(),
                name_for_cmd.clone(),
                app.dialogs.add_metric_api_connector.clone(),
                app.dialogs.add_metric_api_cell.clone(),
                30,
            );
        }

        app.dialogs.add_metric_target = None;
        app.dialogs.add_metric_name.clear();
        app.dialogs.add_metric_kind = MetricKind::Number;
        app.dialogs.add_metric_value.clear();
        app.dialogs.add_metric_unit.clear();
        app.dialogs.add_metric_api_connector.clear();
        app.dialogs.add_metric_api_cell.clear();
    }
}

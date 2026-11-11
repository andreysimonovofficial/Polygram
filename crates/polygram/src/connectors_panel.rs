use crate::app::PolygramApp;
use serde_json::Value;

/// Скрываемый сайдбар «Коннекторы»: настройка каждого источника данных.
pub fn connectors_panel(app: &mut PolygramApp, ctx: &egui::Context) {
    if !app.show_connectors_panel {
        return;
    }

    egui::SidePanel::right("connectors_panel")
        .default_width(420.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("Коннекторы");
            ui.separator();

            let connectors = app.connectors.get_connectors();
            if connectors.is_empty() {
                ui.label("Коннекторы не найдены (проверьте подключение к API).");
                return;
            }

            egui::ScrollArea::vertical()
                .id_source("connectors_scroll")
                .show(ui, |ui| {
                    for connector_id in &connectors {
                        let config = app.connectors.get_connector_config(
                            connector_id,
                            &app.connection.api_client,
                            &app.connection.api_base,
                        );
                        let mut config_value = config
                            .as_ref()
                            .and_then(|c| c.as_object())
                            .cloned()
                            .unwrap_or_default();

                        egui::CollapsingHeader::new(format!("{}", connector_id))
                            .default_open(false)
                            .show(ui, |ui| {
                                ui.separator();

                                match connector_id.as_str() {
                                    "google-sheets" => {
                                        render_google_sheets_config(ui, app, &mut config_value);
                                    }
                                    "google-analytics" => {
                                        render_google_analytics_config(ui, app, &mut config_value);
                                    }
                                    "static" => {
                                        render_static_config(ui, &mut config_value);
                                    }
                                    _ => {
                                        ui.label("Настройки не определены для этого коннектора.");
                                    }
                                }
                            });
                    }
                });

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Закрыть").clicked() {
                    app.show_connectors_panel = false;
                }
            });
        });
}

/// Конфигурация Google Таблиц: прямая ссылка на таблицу (через gws CLI, Apps Script не требуется).
fn render_google_sheets_config(
    ui: &mut egui::Ui,
    app: &mut PolygramApp,
    config: &mut serde_json::Map<String, Value>,
) {
    let mut url = config
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();
    let mut interval = config
        .get("intervalSecs")
        .and_then(|v| v.as_u64())
        .unwrap_or(30);

    ui.label("URL Google Таблицы:");
    ui.label("Пример: https://docs.google.com/spreadsheets/d/1tr5UxsQIUlfs4Td2OdGoNGnKhsYRIvXRQdtrWAvnPvk/edit");
    let url_response = ui.text_edit_singleline(&mut url);
    if url_response.lost_focus() && !url.is_empty() {
        config.insert("url".to_owned(), Value::String(url.clone()));
        config.insert(
            "intervalSecs".to_owned(),
            Value::Number(interval.into()),
        );
        app.connectors.set_connector_config(
            "google-sheets",
            Value::Object(config.clone()),
            &app.connection.api_client,
            &app.connection.api_base,
        );
    }

    ui.horizontal(|ui| {
        ui.label("Интервал опроса (сек):");
        let mut interval_str = interval.to_string();
        if ui.text_edit_singleline(&mut interval_str).lost_focus() {
            if let Ok(v) = interval_str.trim().parse::<u64>() {
                interval = v.max(1);
                config.insert("url".to_owned(), Value::String(url.clone()));
                config.insert(
                    "intervalSecs".to_owned(),
                    Value::Number(interval.into()),
                );
                app.connectors.set_connector_config(
                    "google-sheets",
                    Value::Object(config.clone()),
                    &app.connection.api_client,
                    &app.connection.api_base,
                );
            }
        }
    });

    // Статус подключения.
    let status = app.connectors.connector_status.get("google-sheets").cloned();
    let mut check_connection = false;
    ui.horizontal(|ui| {
        if let Some(s) = status {
            let color = if s.ok {
                egui::Color32::from_rgb(76, 175, 80)
            } else {
                egui::Color32::from_rgb(244, 67, 54)
            };
            let label = match &s.error {
                Some(e) if e == "проверка..." => "проверка подключения...",
                Some(e) => e.as_str(),
                None if s.ok => "подключено",
                None => "ошибка",
            };
            ui.colored_label(color, label);
            if let Some(t) = &s.last_checked {
                ui.label(format!("({})", t));
            }
        }
        if ui.button("Проверить подключение").clicked() {
            check_connection = true;
        }
    });
    if check_connection {
        app.connectors.check_connector_status("google-sheets", &app.connection.api_client);
    }

    // Время последнего опроса по привязкам этого коннектора.
    let latest_poll = app
        .bindings
        .api_bindings
        .values()
        .filter(|b| b.connector == "google-sheets")
        .filter_map(|b| b.last_updated.as_deref())
        .max();
    if let Some(ts) = latest_poll {
        ui.label(format!("Последний опрос: {}", ts));
    }

    ui.separator();
    ui.heading("Настройка Google Таблиц");
    ui.label(
        "1. Откройте свою Google Таблицу и скопируйте ссылку из адресной строки браузера.\n\
         2. Вставьте её в поле «URL Google Таблицы» выше.\n\
         3. Убедитесь, что на компьютере установлен и авторизован gws CLI (@googleworkspace/cli),\n\
         4. Изменения сохраняются автоматически и переживают перезапуск API/GUI.\n\
         5. В привязке показателя укажите ячейку, например: Sheet1!A1",
    );

    ui.separator();
    ui.heading("Как это работает");
    ui.label(
        "Значения читаются напрямую из ячеек таблицы через gws CLI (Google Sheets API).\n\
         Deploy Apps Script не требуется — достаточно ссылки на таблицу и настроенного gws.\n\
         Привязки (объект + запись + ячейка + интервал) хранятся в connectors/bindings.json\n\
         и восстанавливаются после перезапуска API/GUI.",
    );
}

/// Конфигурация Google Analytics: список метрик с копированием.
fn render_google_analytics_config(
    ui: &mut egui::Ui,
    app: &mut PolygramApp,
    config: &mut serde_json::Map<String, Value>,
) {
    let mut property_id = config
        .get("propertyId")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();
    let mut channel_id = config
        .get("channelId")
        .and_then(|v| v.as_str())
        .unwrap_or("MINE")
        .to_owned();
    let mut interval = config
        .get("intervalSecs")
        .and_then(|v| v.as_u64())
        .unwrap_or(300);

    ui.label("Property ID (GA4):");
    let pid_response = ui.text_edit_singleline(&mut property_id);
    if pid_response.lost_focus() && !property_id.is_empty() {
        config.insert("propertyId".to_owned(), Value::String(property_id.clone()));
        config.insert("channelId".to_owned(), Value::String(channel_id.clone()));
        config.insert(
            "intervalSecs".to_owned(),
            Value::Number(interval.into()),
        );
        app.connectors.set_connector_config(
            "google-analytics",
            Value::Object(config.clone()),
            &app.connection.api_client,
            &app.connection.api_base,
        );
    }

    ui.label("Channel ID:");
    let cid_response = ui.text_edit_singleline(&mut channel_id);
    if cid_response.lost_focus() && !channel_id.is_empty() {
        config.insert("propertyId".to_owned(), Value::String(property_id.clone()));
        config.insert("channelId".to_owned(), Value::String(channel_id.clone()));
        config.insert(
            "intervalSecs".to_owned(),
            Value::Number(interval.into()),
        );
        app.connectors.set_connector_config(
            "google-analytics",
            Value::Object(config.clone()),
            &app.connection.api_client,
            &app.connection.api_base,
        );
    }

    ui.horizontal(|ui| {
        ui.label("Интервал опроса (сек):");
        let mut interval_str = interval.to_string();
        if ui.text_edit_singleline(&mut interval_str).lost_focus() {
            if let Ok(v) = interval_str.trim().parse::<u64>() {
                interval = v.max(30);
                config.insert("propertyId".to_owned(), Value::String(property_id.clone()));
                config.insert("channelId".to_owned(), Value::String(channel_id.clone()));
                config.insert(
                    "intervalSecs".to_owned(),
                    Value::Number(interval.into()),
                );
                app.connectors.set_connector_config(
                    "google-analytics",
                    Value::Object(config.clone()),
                    &app.connection.api_client,
                    &app.connection.api_base,
                );
            }
        }
    });

    ui.separator();
    ui.heading("Доступные метрики (GA4)");
    ui.label("Нажмите на метрику, чтобы скопировать её в буфер обмена:");

    let metrics = app.connectors.get_ga_metrics();
    egui::Grid::new("ga_metrics_grid").show(ui, |ui| {
        for (i, metric) in metrics.iter().enumerate() {
            if ui.button(metric).clicked() {
                ui.ctx().copy_text(metric.clone());
            }
            if (i + 1) % 3 == 0 {
                ui.end_row();
            }
        }
    });

    ui.separator();
    ui.label("Формат привязки в показателе: ga4:ga:sessions|2024-01-01|today");
}

/// Конфигурация константы (для теста).
fn render_static_config(ui: &mut egui::Ui, _config: &mut serde_json::Map<String, Value>) {
    ui.label("Константа — вводите любое число в поле «ячейка / метрика» при привязке показателя.");
    ui.label("Пример: 42");
}

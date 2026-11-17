use crate::app::PolygramApp;
use polygram_canvas::Theme;

/// Скрываемый сайдбар «Коннекторы»: настройка каждого источника данных.
pub fn connectors_panel(app: &mut PolygramApp, ui: &mut egui::Ui, theme: &Theme) {
    if !app.show_connectors_panel {
        return;
    }

    egui::Panel::right("connectors_panel")
        .exact_size(420.0)
        .resizable(true)
        .show(ui, |ui| {
            ui.heading("Коннекторы");
            ui.separator();

            let connectors = app.connectors.get_connectors();
            if connectors.is_empty() {
                ui.label("Коннекторы не найдены (проверьте подключение к API).");
                return;
            }

            egui::ScrollArea::vertical()
                .id_salt("connectors_scroll")
                .show(ui, |ui| {
                    for connector_id in &connectors {
                        let config = app.connectors.get_connector_config(
                            connector_id,
                            &app.connection.api_client,
                            &app.connection.api_base,
                        );

                        egui::CollapsingHeader::new(format!("{}", connector_id))
                            .default_open(false)
                            .show(ui, |ui| {
                                // Конфигурация (только для чтения).
                                ui.label("Конфигурация:");
                                ui.monospace(config.map(|v| v.to_string()).unwrap_or_default());

                                ui.separator();

                                // Статус подключения.
                                let status = app.connectors.connector_status.get(connector_id);
                                if let Some(s) = status {
                                    let color = if s.ok {
                                        theme.status_ok
                                    } else {
                                        theme.status_deficit
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
                            });
                    }
                });

            // Статус подключения google-sheets.
            let status = app.connectors.connector_status.get("google-sheets").cloned();
            let mut check_connection = false;
            ui.horizontal(|ui| {
                if let Some(s) = status {
                    let color = if s.ok {
                        theme.status_ok
                    } else {
                        theme.status_deficit
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
                "Коннектор опрашивает Google Таблицу через gws CLI и публикует значения ячеек\n\
                 как показатели в API. Привязка показателя к ячейке (например, Sheet1!A1)\n\
                 позволяет отображать значение прямо на холсте Polygram.",
            );

            ui.separator();
            ui.heading("Примеры привязок");
            ui.monospace("{ \"cmd\": \"add_metric_binding\", \"object\": \"A\", \"dir\": \"output\", \"metric_def\": \"Деньги\", \"connector\": \"google-sheets\", \"cell\": \"Sheet1!A1\" }");
        });
}

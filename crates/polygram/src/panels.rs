use crate::app::PolygramApp;
use polygram_core::Command;

pub fn command_log_panel(app: &PolygramApp, ctx: &egui::Context) {
    egui::SidePanel::right("command_log")
        .default_width(300.0)
        .show(ctx, |ui| {
            ui.heading("Лог команд");
            egui::ScrollArea::vertical().show(ui, |ui| {
                for entry in &app.commands.command_log {
                    ui.label(format!("> {}", entry.cmd));
                    ui.colored_label(
                        if entry.result.starts_with("OK") {
                            egui::Color32::from_rgb(76, 175, 80)
                        } else {
                            egui::Color32::from_rgb(244, 67, 54)
                        },
                        &entry.result,
                    );
                    ui.separator();
                }
            });
            ui.separator();
            ui.label("Примеры команд:");
            ui.monospace("{ \"cmd\": \"add_object\", \"name\": \"Завод А\", \"kind\": \"process\" }");
            ui.monospace("{ \"cmd\": \"add_metric_def\", \"name\": \"Товары\", \"unit\": \"ед.\" }");
            ui.monospace("{ \"cmd\": \"add_metric_entry\", \"object\": \"obj1\", \"dir\": \"output\", \"metric_def\": \"Товары\" }");
            ui.monospace("{ \"cmd\": \"connect\", \"from_object\": \"obj1\", \"from_entry\": \"out1\", \"to_object\": \"obj2\", \"to_entry\": \"request1\" }");
            ui.monospace("{ \"cmd\": \"set_value\", \"object\": \"obj1\", \"entry\": \"out1\", \"value\": 100 }");
            ui.monospace("{ \"cmd\": \"restart_server\" }");
            ui.monospace("{ \"cmd\": \"change_port\", \"port\": 8081 }");
            ui.monospace("{ \"cmd\": \"shutdown\" }");
        });
}

/// Панель подключения к API-серверу.
/// Показывает статус, адрес, последнюю ошибку и лог попыток подключения.
pub fn connection_panel(app: &mut PolygramApp, ctx: &egui::Context) {
    if !app.show_sidebar {
        return;
    }

    egui::SidePanel::left("connection_panel")
        .default_width(360.0)
        .resizable(true)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .id_source("connection_panel")
                .show(ui, |ui| {
                    // API collapsible header.
                    egui::CollapsingHeader::new("API").show(ui, |ui| {
                        // Статус сервера API.
                        let api_running = app.api_process_running();
                        let api_alive = app.connection.api_connected || api_running;
                        ui.horizontal(|ui| {
                            if api_alive {
                                ui.colored_label(egui::Color32::from_rgb(76, 175, 80), "● запущен");
                            } else {
                                ui.colored_label(egui::Color32::from_rgb(244, 67, 54), "● остановлен");
                            }
                        });

                        // Кнопка управления API.
                        ui.horizontal(|ui| {
                            if !api_running && ui.button("Запустить API").clicked() {
                                app.start_api_process();
                            }
                            if api_running && ui.button("Остановить API").clicked() {
                                app.stop_api_process();
                            }
                        });

                        // Показать/скрыть сайдбар Коннекторы.
                        ui.horizontal(|ui| {
                            let label = if app.show_connectors_panel {
                                "Скрыть коннекторы"
                            } else {
                                "Показать коннекторы"
                            };
                            if ui.button(label).clicked() {
                                app.show_connectors_panel = !app.show_connectors_panel;
                            }
                        });

                        ui.separator();

                        // Подключение к API.
                        ui.heading("Подключение к API");
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.label("Адрес:");
                            if ui.text_edit_singleline(&mut app.connection.api_base).lost_focus() {
                                app.try_connect_once();
                            }
                        });
                        ui.horizontal(|ui| {
                            if ui.button("Подключиться / Обновить").clicked() {
                                app.try_connect_once();
                            }
                            if app.connection.api_connected {
                                ui.colored_label(egui::Color32::from_rgb(76, 175, 80), "● онлайн");
                            } else {
                                ui.colored_label(egui::Color32::from_rgb(244, 67, 54), "● офлайн");
                            }
                        });
                        if app.connection.api_connected {
                            ui.horizontal(|ui| {
                                ui.label("Ревизия:");
                                ui.monospace(app.connection.last_rev.to_string());
                            });
                        }
                        if let Some(ref err) = app.connection.api_error {
                            ui.separator();
                            ui.label("Ошибка:");
                            ui.colored_label(egui::Color32::from_rgb(244, 67, 54), err);
                        }

                        ui.separator();

                        // Лог подключения.
                        ui.heading("Лог подключения");
                        egui::ScrollArea::vertical()
                            .id_source("connection_log")
                            .max_height(150.0)
                            .show(ui, |ui| {
                                for entry in &app.connection.connection_log {
                                    let color = if entry.ok {
                                        egui::Color32::from_rgb(76, 175, 80)
                                    } else {
                                        egui::Color32::from_rgb(244, 67, 54)
                                    };
                                    ui.colored_label(color, format!("[{}] {}", entry.time, entry.message));
                                }
                            });

                        ui.separator();

                        // Управление сервером.
                        if app.connection.api_connected {
                            ui.heading("Управление сервером");
                            ui.separator();
                            ui.horizontal(|ui| {
                                if ui.button("Перезапустить сервер").clicked() {
                                    app.apply_cmd(Command::RestartServer);
                                    app.commands.command_log.push(crate::app::CommandLogEntry {
                                        cmd: "{\"cmd\":\"restart_server\"}".to_owned(),
                                        result: "Команда отправлена".to_owned(),
                                    });
                                }
                                if ui.button("Остановить сервер").clicked() {
                                    app.apply_cmd(Command::Shutdown);
                                    app.commands.command_log.push(crate::app::CommandLogEntry {
                                        cmd: "{\"cmd\":\"shutdown\"}".to_owned(),
                                        result: "Команда отправлена".to_owned(),
                                    });
                                }
                            });

                            ui.separator();
                            let mut port_str = "8080".to_owned();
                            ui.horizontal(|ui| {
                                ui.label("Новый порт:");
                                if ui.text_edit_singleline(&mut port_str).lost_focus() {
                                    if let Ok(new_port) = port_str.parse::<u16>() {
                                        app.apply_cmd(Command::ChangePort { port: new_port });
                                    }
                                }
                                if ui.button("Применить порт").clicked() {
                                    if let Ok(new_port) = port_str.parse::<u16>() {
                                        app.apply_cmd(Command::ChangePort { port: new_port });
                                    }
                                }
                            });
                            ui.separator();
                        }
                    });

                    // MCP collapsible header.
                    egui::CollapsingHeader::new("MCP").show(ui, |ui| {
                        // Статус сервера MCP.
                        let mcp_running = app.mcp_process_running();
                        ui.horizontal(|ui| {
                            if mcp_running {
                                ui.colored_label(egui::Color32::from_rgb(76, 175, 80), "● запущен");
                            } else {
                                ui.colored_label(egui::Color32::from_rgb(244, 67, 54), "● остановлен");
                            }
                        });
                        ui.horizontal(|ui| {
                            if mcp_running && ui.button("Остановить MCP").clicked() {
                                app.stop_mcp_process();
                            }
                            if mcp_running && ui.button("Перезапустить MCP").clicked() {
                                app.restart_mcp_process();
                            }
                        });

                        ui.separator();

                        // Claude Desktop.
                        ui.heading("Claude Desktop");
                        ui.separator();
                        let mcp_exe = std::env::current_exe()
                            .ok()
                            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
                            .join("polygram-mcp.exe");
                        let mcp_exe_str = mcp_exe.display().to_string();
                        let mcp_exe_json = mcp_exe_str.replace('\\', "\\\\");
                        let claude_config = format!(
                            "{{\n  \"mcpServers\": {{\n    \"polygram\": {{\n      \"command\": \"{}\",\n      \"args\": [\"--http\", \"http://127.0.0.1:8080\"]\n    }}\n  }}\n}}",
                            mcp_exe_json
                        );
                        egui::ScrollArea::vertical()
                            .id_source("claude_config")
                            .max_height(100.0)
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::multiline(&mut claude_config.clone())
                                        .code_editor()
                                        .desired_rows(5)
                                        .interactive(false),
                                );
                            });
                        if ui.button("Копировать конфиг Claude Desktop").clicked() {
                            ctx.copy_text(claude_config);
                        }

                        ui.separator();

                        // Open WebUI.
                        ui.heading("Open WebUI");
                        ui.separator();
                        ui.label("Имя сервера:");
                        ui.monospace("polygram");
                        ui.label("Тип:");
                        ui.monospace("stdio");
                        ui.label("Команда:");
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut mcp_exe_str.clone())
                                    .code_editor()
                                    .interactive(false),
                            );
                            if ui.button("Копировать").clicked() {
                                ctx.copy_text(mcp_exe_str.clone());
                            }
                        });
                        ui.label("Аргументы:");
                        ui.monospace("--http http://127.0.0.1:8080");
                        ui.separator();

                        // Примеры команд.
                        ui.heading("Примеры команд");
                        egui::ScrollArea::vertical()
                            .id_source("example_commands")
                            .max_height(100.0)
                            .show(ui, |ui| {
                                for cmd in [
                                    r#"{"cmd":"add_object","id":"A","name":"Процесс А","kind":"process"}"#,
                                    r#"{"cmd":"add_metric_def","id":"Деньги","name":"Деньги","kind":"number","unit":"$"}"#,
                                    r#"{"cmd":"add_metric_entry","object":"A","dir":"output","metric_def":"Деньги","name":"Деньги","value":100.0}"#,
                                    r#"{"cmd":"restart_server"}"#,
                                    r#"{"cmd":"change_port","port":8081}"#,
                                ] {
                                    ui.monospace(cmd);
                                }
                            });
                    });
                });
        });
}

/// Отдельный сайдбар с общим чатом всех участников (user / claude / qwen / http).
pub fn chat_panel(app: &mut PolygramApp, ctx: &egui::Context) {
    egui::SidePanel::right("chat_panel")
        .default_width(300.0)
        .resizable(true)
        .show(ctx, |ui| {
            crate::app::chat::render(app, ui);
        });
}

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

            let connectors = app.get_connectors();
            if connectors.is_empty() {
                ui.label("Коннекторы не найдены (проверьте подключение к API).");
                return;
            }

            egui::ScrollArea::vertical()
                .id_source("connectors_scroll")
                .show(ui, |ui| {
                    for connector_id in &connectors {
                        let config = app.get_connector_config(connector_id);
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

/// Конфигурация Google Таблиц: URL Apps Script + инструкция.
fn render_google_sheets_config(ui: &mut egui::Ui, app: &mut PolygramApp, config: &mut serde_json::Map<String, serde_json::Value>) {
    let mut url = config
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();
    let mut interval = config
        .get("intervalSecs")
        .and_then(|v| v.as_u64())
        .unwrap_or(30);

    ui.label("URL Apps Script:");
    let url_response = ui.text_edit_singleline(&mut url);
    if url_response.lost_focus() && !url.is_empty() {
        config.insert("url".to_owned(), serde_json::Value::String(url.clone()));
        config.insert(
            "intervalSecs".to_owned(),
            serde_json::Value::Number(interval.into()),
        );
        app.set_connector_config("google-sheets", serde_json::Value::Object(config.clone()));
    }

    ui.horizontal(|ui| {
        ui.label("Интервал опроса (сек):");
        let mut interval_str = interval.to_string();
        if ui.text_edit_singleline(&mut interval_str).lost_focus() {
            if let Ok(v) = interval_str.trim().parse::<u64>() {
                interval = v.max(1);
                config.insert("url".to_owned(), serde_json::Value::String(url.clone()));
                config.insert(
                    "intervalSecs".to_owned(),
                    serde_json::Value::Number(interval.into()),
                );
                app.set_connector_config("google-sheets", serde_json::Value::Object(config.clone()));
            }
        }
    });

    // Статус подключения.
    let status = app.connector_status.get("google-sheets").cloned();
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
        app.check_connector_status("google-sheets");
    }

    ui.separator();
    ui.heading("App Script для Google Таблиц");
    ui.label("Скопируйте код ниже в редактор Apps Script (https://script.google.com):");

    const APPS_SCRIPT: &str = r#"function doGet(e) {
  var cell = e.parameter.cell;
  if (!cell) return ContentService.createTextOutput("no cell");
  var sheet = SpreadsheetApp.openByUrl(PropertiesService.getScriptProperties().getProperty("SHEET_URL"));
  var range = sheet.getRange(cell);
  var value = range.getValue();
  if (typeof value === "boolean") value = value ? 1 : 0;
  return ContentService.createTextOutput(JSON.stringify({value: value}));
}"#;

    ui.add(
        egui::TextEdit::multiline(&mut APPS_SCRIPT.to_owned())
            .code_editor()
            .desired_rows(12)
            .interactive(false),
    );

    if ui.button("Копировать App Script").clicked() {
        ui.ctx().copy_text(APPS_SCRIPT.to_owned());
    }

    ui.separator();
    ui.heading("Инструкция");
    ui.label(
        "1. Откройте https://script.google.com и создайте новый проект.\n\
         2. Вставьте код App Script выше.\n\
         3. Добавьте свойство скрипта SHEET_URL со ссылкой на вашу Google Таблицу.\n\
         4. Разверните как Веб-приложение (Deploy → New deployment → Web app).\n\
         5. Скопируйте URL веб-приложения в поле «URL Apps Script» выше.\n\
         6. В привязке показателя укажите ячейку, например: Sheet1!A1",
    );
}

/// Конфигурация Google Analytics: список метрик с копированием.
fn render_google_analytics_config(ui: &mut egui::Ui, app: &mut PolygramApp, config: &mut serde_json::Map<String, serde_json::Value>) {
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
        config.insert("propertyId".to_owned(), serde_json::Value::String(property_id.clone()));
        config.insert("channelId".to_owned(), serde_json::Value::String(channel_id.clone()));
        config.insert(
            "intervalSecs".to_owned(),
            serde_json::Value::Number(interval.into()),
        );
        app.set_connector_config("google-analytics", serde_json::Value::Object(config.clone()));
    }

    ui.label("Channel ID:");
    let cid_response = ui.text_edit_singleline(&mut channel_id);
    if cid_response.lost_focus() && !channel_id.is_empty() {
        config.insert("propertyId".to_owned(), serde_json::Value::String(property_id.clone()));
        config.insert("channelId".to_owned(), serde_json::Value::String(channel_id.clone()));
        config.insert(
            "intervalSecs".to_owned(),
            serde_json::Value::Number(interval.into()),
        );
        app.set_connector_config("google-analytics", serde_json::Value::Object(config.clone()));
    }

    ui.horizontal(|ui| {
        ui.label("Интервал опроса (сек):");
        let mut interval_str = interval.to_string();
        if ui.text_edit_singleline(&mut interval_str).lost_focus() {
            if let Ok(v) = interval_str.trim().parse::<u64>() {
                interval = v.max(30);
                config.insert("propertyId".to_owned(), serde_json::Value::String(property_id.clone()));
                config.insert("channelId".to_owned(), serde_json::Value::String(channel_id.clone()));
                config.insert(
                    "intervalSecs".to_owned(),
                    serde_json::Value::Number(interval.into()),
                );
                app.set_connector_config("google-analytics", serde_json::Value::Object(config.clone()));
            }
        }
    });

    ui.separator();
    ui.heading("Доступные метрики (GA4)");
    ui.label("Нажмите на метрику, чтобы скопировать её в буфер обмена:");

    let metrics = app.get_ga_metrics();
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
fn render_static_config(ui: &mut egui::Ui, _config: &mut serde_json::Map<String, serde_json::Value>) {
    ui.label("Константа — вводите любое число в поле «ячейка / метрика» при привязке показателя.");
    ui.label("Пример: 42");
}

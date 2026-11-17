use crate::app::PolygramApp;
use polygram_core::Command;
use polygram_canvas::Theme;

/// Панель подключения к API-серверу.
/// Показывает статус, адрес, последнюю ошибку и лог попыток подключения.
pub fn connection_panel(app: &mut PolygramApp, ui: &mut egui::Ui, theme: &Theme) {
    if !app.show_sidebar {
        return;
    }

    egui::Panel::left("connection_panel")
        .exact_size(360.0)
        .resizable(true)
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("connection_panel")
                .show(ui, |ui| {
                    // API collapsible header.
                    egui::CollapsingHeader::new("API").show(ui, |ui| {
                        // Статус сервера API.
                        let api_running = app.api_process_running();
                        let api_alive = app.connection.api_connected || api_running;
                        ui.horizontal(|ui| {
                            if api_alive {
                                ui.colored_label(theme.status_ok, "● запущен");
                            } else {
                                ui.colored_label(theme.status_deficit, "● остановлен");
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
                                ui.colored_label(theme.status_ok, "● онлайн");
                            } else {
                                ui.colored_label(theme.status_deficit, "● офлайн");
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
                            ui.colored_label(theme.status_deficit, err);
                        }

                        ui.separator();

                        // Лог подключения.
                        ui.heading("Лог подключения");
                        egui::ScrollArea::vertical()
                            .id_salt("connection_log")
                            .max_height(150.0)
                            .show(ui, |ui| {
                                for entry in &app.connection.connection_log {
                                    let color = if entry.ok {
                                        theme.status_ok
                                    } else {
                                        theme.status_deficit
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
                                    app.add_event_log("warn", "command", "Перезапуск API-сервера", None);
                                }
                                if ui.button("Остановить сервер").clicked() {
                                    app.apply_cmd(Command::Shutdown);
                                    app.commands.command_log.push(crate::app::CommandLogEntry {
                                        cmd: "{\"cmd\":\"shutdown\"}".to_owned(),
                                        result: "Команда отправлена".to_owned(),
                                    });
                                    app.add_event_log("warn", "command", "Остановка API-сервера", None);
                                }
                            });

                            ui.separator();
                            let mut port_str = "8080".to_owned();
                            ui.horizontal(|ui| {
                                ui.label("Новый порт:");
                                if ui.text_edit_singleline(&mut port_str).lost_focus() {
                                    if let Ok(new_port) = port_str.parse::<u16>() {
                                        app.apply_cmd(Command::ChangePort { port: new_port });
                                        app.add_event_log("info", "command", &format!("Смена порта API на {new_port}"), None);
                                    }
                                }
                                if ui.button("Применить порт").clicked() {
                                    if let Ok(new_port) = port_str.parse::<u16>() {
                                        app.apply_cmd(Command::ChangePort { port: new_port });
                                        app.add_event_log("info", "command", &format!("Смена порта API на {new_port}"), None);
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
                                ui.colored_label(theme.status_ok, "● запущен");
                            } else {
                                ui.colored_label(theme.status_deficit, "● остановлен");
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
                            .id_salt("claude_config")
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
                            ui.ctx().copy_text(claude_config);
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
                                ui.ctx().copy_text(mcp_exe_str.clone());
                            }
                        });
                        ui.label("Аргументы:");
                        ui.monospace("--http http://127.0.0.1:8080");
                        ui.separator();

                        // Примеры команд.
                        ui.heading("Примеры команд");
                        egui::ScrollArea::vertical()
                            .id_salt("example_commands")
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

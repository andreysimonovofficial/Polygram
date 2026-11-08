use crate::app::PolygramApp;
use polygram_core::Command;

pub fn command_log_panel(app: &PolygramApp, ctx: &egui::Context) {
    egui::SidePanel::right("command_log")
        .default_width(300.0)
        .show(ctx, |ui| {
            ui.heading("Лог команд");
            egui::ScrollArea::vertical().show(ui, |ui| {
                for entry in &app.command_log {
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

/// Панель статуса API-сервера.
/// Показывает подключение к polygram-api (работает 24/7), ревизию, порт.
pub fn mcp_panel(app: &mut PolygramApp, ctx: &egui::Context) {
    if !app.show_mcp_panel {
        return;
    }

    egui::SidePanel::left("mcp_panel")
        .default_width(340.0)
        .show(ctx, |ui| {
            ui.heading("API сервер (polygram-api)");
            ui.separator();

            // Статус подключения.
            ui.horizontal(|ui| {
                ui.label("HTTP API:");
                if app.api_connected {
                    ui.colored_label(egui::Color32::from_rgb(76, 175, 80), "подключён");
                } else {
                    ui.colored_label(egui::Color32::from_rgb(244, 67, 54), "недоступен");
                }
            });
            ui.horizontal(|ui| {
                ui.label("Адрес:");
                ui.monospace(&app.api_base);
            });
            ui.horizontal(|ui| {
                ui.label("Ревизия:");
                ui.monospace(app.last_rev.to_string());
            });
            ui.separator();

            // Кнопки администрирования (чедние те же HTTP-команды).
            ui.heading("Управление сервером");
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Перезапустить сервер").clicked() {
                    app.apply_cmd(Command::RestartServer);
                    app.command_log.push(crate::app::CommandLogEntry {
                        cmd: "{\"cmd\":\"restart_server\"}".to_owned(),
                        result: "Команда отправлена".to_owned(),
                    });
                }
                if ui.button("Остановить сервер").clicked() {
                    app.apply_cmd(Command::Shutdown);
                    app.command_log.push(crate::app::CommandLogEntry {
                        cmd: "{\"cmd\":\"shutdown\"}".to_owned(),
                        result: "Команда отправлена".to_owned(),
                    });
                }
            });

            ui.separator();

            // Смена порта.
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

            // Путь к MCP-серверу для Claude Desktop.
            let mcp_exe = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
                .join("polygram-mcp.exe");
            let mcp_exe_str = mcp_exe.display().to_string();
            let mcp_exe_json = mcp_exe_str.replace('\\', "\\\\");

            ui.heading("Claude Desktop");
            ui.separator();
            let claude_config = format!(
                "{{\n  \"mcpServers\": {{\n    \"polygram\": {{\n      \"command\": \"{}\",\n      \"args\": [\"--http\", \"http://127.0.0.1:8080\"]\n    }}\n  }}\n}}",
                mcp_exe_json
            );
            egui::ScrollArea::vertical()
                .max_height(120.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut claude_config.clone())
                            .code_editor()
                            .desired_rows(6)
                            .interactive(false),
                    );
                });
            if ui.button("Копировать конфиг Claude Desktop").clicked() {
                ctx.copy_text(claude_config);
            }

            ui.separator();

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

            ui.heading("Примеры команд");
            ui.separator();
            egui::ScrollArea::vertical()
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
}

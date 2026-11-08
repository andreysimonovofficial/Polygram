use crate::app::PolygramApp;
use std::env;
use std::sync::Arc;

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
        });
}

/// Боковая панель управления встроенным MCP / HTTP сервером.
/// Вкладка в самом приложении: статус сервера, порт, перезапуск, копирование состояния.
pub fn mcp_panel(app: &mut PolygramApp, ctx: &egui::Context) {
    if !app.show_mcp_panel {
        return;
    }

    let server_control = app.server_control.clone();

    egui::SidePanel::left("mcp_panel")
        .default_width(340.0)
        .show(ctx, |ui| {
            ui.heading("MCP сервер");
            ui.separator();

            // Читаем состояние.
            let (mcp_running, http_port) = {
                let sc = server_control.lock().unwrap();
                let running = sc.mcp_is_running();
                let port = *sc.http_port.lock().unwrap();
                (running, port)
            };

            // Статус.
            ui.horizontal(|ui| {
                ui.label("MCP (stdio):");
                if mcp_running {
                    ui.colored_label(egui::Color32::from_rgb(76, 175, 80), "запущен");
                } else {
                    ui.colored_label(egui::Color32::from_rgb(244, 67, 54), "остановлен");
                }
            });
            ui.horizontal(|ui| {
                ui.label("HTTP API:");
                if mcp_running {
                    ui.colored_label(egui::Color32::from_rgb(76, 175, 80), format!("порт {http_port}"));
                } else {
                    ui.colored_label(egui::Color32::from_rgb(244, 67, 54), "остановлен");
                }
            });
            ui.separator();

            // Кнопки управления.
            ui.horizontal(|ui| {
                if ui.button("Перезапустить").clicked() {
                    server_control.lock().unwrap().restart_mcp();
                    server_control.lock().unwrap().restart_http();
                }
                if ui.button("Стоп").clicked() {
                    server_control.lock().unwrap().stop_mcp();
                }
                if ui.button("Пуск").clicked() {
                    let port = {
                        let sc = server_control.lock().unwrap();
                        let p = *sc.http_port.lock().unwrap();
                        p
                    };
                    server_control.lock().unwrap().start_mcp();
                    server_control.lock().unwrap().start_http(port);
                }
            });

            ui.separator();

            // Смена порта HTTP.
            let mut port_str = {
                let sc = server_control.lock().unwrap();
                let port = *sc.http_port.lock().unwrap();
                port.to_string()
            };
            ui.horizontal(|ui| {
                ui.label("Порт HTTP:");
                if ui.text_edit_singleline(&mut port_str).lost_focus() {
                    if let Ok(new_port) = port_str.parse::<u16>() {
                        let guard = server_control.lock().unwrap();
                        *guard.http_port.lock().unwrap() = new_port;
                        drop(guard);
                        server_control.lock().unwrap().restart_http();
                    }
                }
                if ui.button("Применить").clicked() {
                    if let Ok(new_port) = port_str.parse::<u16>() {
                        let guard = server_control.lock().unwrap();
                        *guard.http_port.lock().unwrap() = new_port;
                        drop(guard);
                        server_control.lock().unwrap().restart_http();
                    }
                }
            });

            ui.separator();

            // Копирование состояния.
            ui.label("Состояние:");
            let api = {
                let sc = server_control.lock().unwrap();
                Arc::clone(&sc.api)
            };
            if ui.button("Копировать объекты (JSON)").clicked() {
                let objects = api.objects();
                if let Ok(json) = serde_json::to_string_pretty(&objects) {
                    ctx.copy_text(json);
                }
            }
            if ui.button("Копировать статистику (JSON)").clicked() {
                let stats = api.stats();
                if let Ok(json) = serde_json::to_string_pretty(&stats) {
                    ctx.copy_text(json);
                }
            }

            ui.separator();

            // Путь к MCP-серверу.
            let mcp_exe = env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| env::current_dir().unwrap_or_default())
                .join("polygram-mcp.exe");
            let mcp_exe_str = mcp_exe.display().to_string();
            let mcp_exe_json = mcp_exe_str.replace('\\', "\\\\");

            // Конфиг для Claude Desktop.
            ui.heading("Claude Desktop");
            ui.separator();
            let claude_config = format!(
                "{{\n  \"mcpServers\": {{\n    \"polygram\": {{\n      \"command\": \"{}\",\n      \"args\": []\n    }}\n  }}\n}}",
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

            // Конфиг для Open WebUI.
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
            ui.monospace("(пусто)");
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
                    ] {
                        ui.monospace(cmd);
                    }
                });
        });
}

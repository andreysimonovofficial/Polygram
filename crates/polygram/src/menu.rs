use crate::app::PolygramApp;
use polygram_canvas::Camera;
use polygram_core::Command;
use std::sync::Arc;

pub fn top_menu(app: &mut PolygramApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("menu").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("Файл", |ui| {
                if ui.button("Новый граф").clicked() {
                    ui.close_menu();
                }
                if ui.button("Загрузить .pg").clicked() {
                    ui.close_menu();
                }
                if ui.button("Сохранить .pg").clicked() {
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Выход").clicked() {
                    ui.close_menu();
                }
            });

            ui.menu_button("Вид", |ui| {
                if ui.button("Автораскладка").clicked() {
                    app.apply_cmd(Command::AutoLayout);
                    ui.close_menu();
                }
                if ui.button("Перемаршрутизировать").clicked() {
                    app.apply_cmd(Command::RerouteAll);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Сброс камеры").clicked() {
                    app.canvas.camera = Camera::new();
                    ui.close_menu();
                }
            });

            ui.separator();

            // Кнопка MCP — открывает окно управления встроенным сервером.
            let mcp_label = if app.show_mcp_window {
                "MCP (окно)"
            } else {
                "MCP (окно)"
            };
            if ui.button(mcp_label).clicked() {
                app.show_mcp_window = !app.show_mcp_window;
            }

            // Вкладка MCP — боковая панель в приложении.
            let mcp_panel_label = if app.show_mcp_panel {
                "MCP (панель)"
            } else {
                "MCP (панель)"
            };
            if ui.button(mcp_panel_label).clicked() {
                app.show_mcp_panel = !app.show_mcp_panel;
            }

            ui.separator();
            if ui.button("Команды (Ctrl+P)").clicked() {
                app.show_command_palette = true;
            }
        });
    });
}

/// Окно управления встроенным MCP / HTTP сервером.
pub fn mcp_window(
    app: &mut PolygramApp,
    ctx: &egui::Context,
) {
    let server_control = app.server_control.clone();

    // Читаем текущее состояние (блокируем Mutex только на чтение).
    let (mcp_running, http_port) = {
        let sc = server_control.lock().unwrap();
        let running = sc.mcp_is_running();
        let port = *sc.http_port.lock().unwrap();
        (running, port)
    };

    egui::Window::new("MCP сервер")
        .open(&mut app.show_mcp_window)
        .default_size([520.0, 340.0])
        .show(ctx, |ui| {
            ui.heading("MCP / HTTP сервер Polygram");
            ui.separator();

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
                    ui.colored_label(egui::Color32::from_rgb(76, 175, 80), format!("запущен на порту {http_port}"));
                } else {
                    ui.colored_label(egui::Color32::from_rgb(244, 67, 54), "остановлен");
                }
            });
            ui.separator();

            // Кнопки управления (используют Arc<Mutex<ServerControl>>).
            ui.horizontal(|ui| {
                if ui.button("Перезапустить сервер").clicked() {
                    server_control.lock().unwrap().restart_mcp();
                    server_control.lock().unwrap().restart_http();
                }
                if ui.button("Остановить").clicked() {
                    server_control.lock().unwrap().stop_mcp();
                    server_control.lock().unwrap().stop_http();
                }
                if ui.button("Запустить").clicked() {
                    server_control.lock().unwrap().start_mcp();
                    let port = {
                        let sc = server_control.lock().unwrap();
                        let p = *sc.http_port.lock().unwrap();
                        p
                    };
                    server_control.lock().unwrap().start_http(port);
                }
            });

            ui.separator();

            // Смена порта HTTP.
            let mut port_str = http_port.to_string();
            ui.horizontal(|ui| {
                ui.label("Порт HTTP:");
                if ui.text_edit_singleline(&mut port_str).lost_focus() {
                    if let Ok(new_port) = port_str.parse::<u16>() {
                        let mut port_guard = server_control.lock().unwrap();
                        *port_guard.http_port.lock().unwrap() = new_port;
                        drop(port_guard);
                        server_control.lock().unwrap().restart_http();
                    }
                }
                if ui.button("Применить порт").clicked() {
                    if let Ok(new_port) = port_str.parse::<u16>() {
                        let mut port_guard = server_control.lock().unwrap();
                        *port_guard.http_port.lock().unwrap() = new_port;
                        drop(port_guard);
                        server_control.lock().unwrap().restart_http();
                    }
                }
            });

            ui.separator();

            // Данные для копирования.
            ui.label("Данные состояния:");
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
            ui.label("Подключение MCP-клиентов (stdio):");
            ui.monospace("polygram.exe — встроенный сервер, stdin/stdout");
            ui.label("Claude Desktop: examples/mcp/claude_desktop_config.json");
        });
}

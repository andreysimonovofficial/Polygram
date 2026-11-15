use crate::app::PolygramApp;
use egui::Color32;
use polygram_canvas::{Camera, GridMode, Theme};
use polygram_core::Command;

pub fn top_menu(app: &mut PolygramApp, ui: &mut egui::Ui, theme: &Theme) {
    egui::Panel::top("menu").show(ui, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("Файл", |ui| {
                if ui.button("Новый граф").clicked() {
                    
                    if app.dirty {
                        app.close_confirmation_pending = true;
                    } else {
                        app.connection.snapshot = None;
                        app.connection.last_rev = 0;
                        app.mark_clean();
                        app.last_opened_file = None;
                        app.try_connect_once();
                    }
                }
                if ui.button("Открыть проект…").clicked() {
                    
                    app.open_project_requested = true;
                }
                if ui.button("Сохранить проект").clicked() {
                    
                    if app.last_opened_file.is_some() {
                        app.save_project_requested = true;
                    } else {
                        app.save_project_as_requested = true;
                    }
                }
                if ui.button("Сохранить проект как…").clicked() {
                    
                    app.save_project_as_requested = true;
                }
                ui.separator();
                if ui.button("Загрузить .pg (отдел)").clicked() {
                    
                    app.import_requested = true;
                }
                if ui.button("Сохранить .pg (отдел)").clicked() {
                    
                    app.export_requested = true;
                }
                ui.separator();
                if ui.button("Экспортировать отдел").clicked() {
                    
                    app.export_department_requested = true;
                }
                if ui.button("Импортировать отдел").clicked() {
                    
                    app.import_department_requested = true;
                }
                ui.separator();
                if ui.button("Выход").clicked() {
                    
                    if app.dirty {
                        app.close_confirmation_pending = true;
                    } else {
                        std::process::exit(0);
                    }
                }
            });

            ui.menu_button("Вид", |ui| {
            if ui.button("Автораскладка").clicked() {
                app.apply_cmd(Command::AutoLayout);
                app.add_event_log("info", "command", "Запущена автораскладка графа (меню)", None);
                
            }
                ui.separator();
                if ui.button("Сброс камеры").clicked() {
                    app.canvas.camera = Camera::new();
                    
                }
                ui.separator();
            ui.menu_button("Сетка", |ui| {
                ui.label("Оформление");
                if ui.button("Цвет сетки").clicked() {
                    app.show_color_picker = true;
                }
                if app.show_color_picker {
                    let mut open = app.show_color_picker;
                    egui::Window::new("Цвет сетки")
                        .open(&mut open)
                        .show(ui.ctx(), |ui| {
                            let mut color =
                                Color32::from_rgb(app.grid_color[0], app.grid_color[1], app.grid_color[2]);
                            egui::widgets::color_picker::color_picker_color32(
                                ui,
                                &mut color,
                                egui::widgets::color_picker::Alpha::Opaque,
                            );
                            app.grid_color = [color.r(), color.g(), color.b()];
                            app.canvas.theme.grid_color = color;
                            app.ui_config_save();
                        });
                    app.show_color_picker = open;
                }
                ui.separator();
                ui.label("Отображение");
                let mut changed = false;
                changed |= ui.radio_value(&mut app.grid_mode, GridMode::None, "Отключить").changed();
                changed |= ui.radio_value(&mut app.grid_mode, GridMode::Lines, "Из линий").changed();
                changed |= ui.radio_value(&mut app.grid_mode, GridMode::Dots, "Из точки").changed();
                if changed {
                    app.ui_config_save();
                }
            });
            if ui.checkbox(&mut app.show_cursor_coords, "Показать координаты курсора").changed() {}
                if ui.checkbox(&mut app.show_connectors_panel, "Коннекторы").changed() {
                    app.ui_config_save();
                }
            });

            ui.separator();

            // Показать/скрыть боковые панели.
            let mut changed = false;
            if ui.checkbox(&mut app.show_sidebar, "Боковая панель API").changed() {
                changed = true;
            }
            if ui.checkbox(&mut app.show_command_log_panel, "Лог команд").changed() {
                changed = true;
            }
            if ui.checkbox(&mut app.show_frontend_log_panel, "Лог событий фронтенда").changed() {
                changed = true;
            }
            if ui.checkbox(&mut app.show_chat_panel, "Чат").changed() {
                changed = true;
            }
            if changed {
                app.ui_config_save();
            }

            ui.separator();
            if ui.button("Команды (Ctrl+P)").clicked() {
                app.commands.show_command_palette = true;
            }
        });
    });
}

use crate::app::PolygramApp;
use polygram_core::Command;
use polygram_canvas::Camera;

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
            if ui.button("Команды (Ctrl+P)").clicked() {
                app.show_command_palette = true;
            }
        });
    });
}

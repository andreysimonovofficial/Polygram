use crate::app::PolygramApp;
use polygram_core::Command;

pub fn command_palette(app: &mut PolygramApp, ctx: &egui::Context) {
    if !app.commands.show_command_palette {
        return;
    }

    egui::Window::new("Палитра команд")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 40.0))
        .show(ctx, |ui| {
            ui.label("Введите команду JSON, например:");
            ui.monospace("{ \"cmd\": \"add_node\", \"name\": \"Завод А\", \"kind\": \"process\" }");
            ui.separator();
            ui.text_edit_singleline(&mut app.commands.command_input);
            if ui.button("Выполнить").clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter))
            {
                match serde_json::from_str::<Command>(&app.commands.command_input) {
                    Ok(cmd) => {
                        app.apply_cmd(cmd);
                        app.commands.command_input.clear();
                        app.commands.show_command_palette = false;
                    }
                    Err(_) => {
                        app.last_status = "Неверный JSON команды".to_owned();
                    }
                }
            }
            if ui.button("Отмена").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape))
            {
                app.commands.show_command_palette = false;
            }
        });
}

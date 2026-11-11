use crate::app::PolygramApp;

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

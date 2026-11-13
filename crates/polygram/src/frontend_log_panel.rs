use crate::app::PolygramApp;

pub fn frontend_log_panel(app: &mut PolygramApp, ctx: &egui::Context) {
    egui::SidePanel::right("frontend_log")
        .default_width(340.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.heading("Лог событий фронтенда");
            ui.separator();

            egui::ScrollArea::vertical()
                .id_source("frontend_log_scroll")
                .show(ui, |ui| {
                    for entry in &app.commands.event_log {
                        let level_color = match entry.level.as_str() {
                            "error" => egui::Color32::from_rgb(244, 67, 54),
                            "warn" => egui::Color32::from_rgb(255, 152, 0),
                            "success" => egui::Color32::from_rgb(76, 175, 80),
                            _ => egui::Color32::from_rgb(33, 150, 243),
                        };

                        ui.horizontal(|ui| {
                            ui.label(entry.time.as_str());
                            ui.colored_label(level_color, format!("[{}]", entry.level));
                            ui.colored_label(
                                egui::Color32::from_rgb(158, 158, 158),
                                format!("<{}>", entry.category),
                            );
                            ui.label(&entry.message);
                        });
                        if let Some(detail) = &entry.detail {
                            ui.indent("detail", |ui| {
                                ui.monospace(detail);
                            });
                        }
                        ui.separator();
                    }
                });

            ui.separator();
            if ui.button("Очистить лог").clicked() {
                app.commands.event_log.clear();
            }
        });
}

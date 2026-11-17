use crate::app::PolygramApp;
use polygram_canvas::Theme;

pub fn frontend_log_panel(app: &mut PolygramApp, ui: &mut egui::Ui, theme: &Theme) {
    egui::Panel::right("frontend_log")
        .default_size(340.0)
        .min_size(96.0)
        .resizable(true)
        .show(ui, |ui| {
            ui.heading("Лог событий фронтенда");
            ui.separator();

            egui::ScrollArea::vertical()
                .id_salt("frontend_log_scroll")
                .show(ui, |ui| {
                    for entry in &app.commands.event_log {
                        let level_color = match entry.level.as_str() {
                            "error" => theme.status_deficit,
                            "warn" => theme.status_warning,
                            "success" => theme.status_ok,
                            _ => theme.accent_secondary,
                        };

                        ui.horizontal(|ui| {
                            ui.label(entry.time.as_str());
                            ui.colored_label(level_color, format!("[{}]", entry.level));
                            ui.colored_label(
                                theme.text_muted,
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

use crate::app::PolygramApp;
use polygram_canvas::Theme;

pub use crate::command_log_panel::command_log_panel;
pub use crate::connection_panel::connection_panel;
pub use crate::connectors_panel::connectors_panel;
pub use crate::frontend_log_panel::frontend_log_panel;

/// Скрываемый сайдбар с общим чатом всех участников (user / claude / qwen / http).
pub fn chat_panel(app: &mut PolygramApp, ui: &mut egui::Ui, theme: &Theme) {
    egui::Panel::right("chat_panel")
        .exact_size(300.0)
        .resizable(true)
        .show(ui, |ui| {
            crate::app::chat::render(app, ui, theme);
        });
}

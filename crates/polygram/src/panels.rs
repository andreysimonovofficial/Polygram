use crate::app::PolygramApp;

pub use crate::command_log_panel::command_log_panel;
pub use crate::connection_panel::connection_panel;
pub use crate::connectors_panel::connectors_panel;
pub use crate::frontend_log_panel::frontend_log_panel;

/// Скрываемый сайдбар с общим чатом всех участников (user / claude / qwen / http).
pub fn chat_panel(app: &mut PolygramApp, ctx: &egui::Context) {
    egui::SidePanel::right("chat_panel")
        .default_width(300.0)
        .resizable(true)
        .show(ctx, |ui| {
            crate::app::chat::render(app, ui);
        });
}

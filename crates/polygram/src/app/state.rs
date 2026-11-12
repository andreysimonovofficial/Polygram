use crate::app::PolygramApp;

pub struct CommandLogEntry {
    pub cmd: String,
    pub result: String,
}

impl PolygramApp {
    pub fn ui_config_save(&self) {
        let ui_config = crate::app::ui_config::UiConfig {
            show_sidebar: self.show_sidebar,
            show_command_log_panel: self.show_command_log_panel,
            show_chat_panel: self.show_chat_panel,
            show_connectors_panel: self.show_connectors_panel,
            show_grid: self.show_grid,
            show_cursor_coords: self.show_cursor_coords,
        };
        ui_config.save();
    }

    pub fn poll_chat(&mut self) {
        self.chat.poll();
    }

    pub fn add_connection_log(&mut self, ok: bool, message: &str) {
        self.connection.add_connection_log(ok, message);
    }
}

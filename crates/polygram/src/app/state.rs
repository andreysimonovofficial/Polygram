use crate::app::PolygramApp;
use chrono::Local;

pub struct CommandLogEntry {
    pub cmd: String,
    pub result: String,
}

pub struct EventLogEntry {
    pub time: String,
    pub level: String,
    pub category: String,
    pub message: String,
    pub detail: Option<String>,
}

impl PolygramApp {
    pub fn add_event_log(&mut self, level: &str, category: &str, message: &str, detail: Option<&str>) {
        let time = Local::now().format("%H:%M:%S").to_string();
        self.commands.event_log.push(EventLogEntry {
            time,
            level: level.to_owned(),
            category: category.to_owned(),
            message: message.to_owned(),
            detail: detail.map(|s| s.to_owned()),
        });
        if self.commands.event_log.len() > 500 {
            self.commands.event_log.remove(0);
        }
    }
    pub fn ui_config_save(&self) {
        let ui_config = crate::app::ui_config::UiConfig {
            show_sidebar: self.show_sidebar,
            show_command_log_panel: self.show_command_log_panel,
            show_frontend_log_panel: self.show_frontend_log_panel,
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

use polygram_canvas::Canvas;
use polygram_core::Published;
use reqwest::blocking::Client;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

pub mod connection;
pub mod chat;
pub mod dialogs;
pub mod drag;
pub mod commands;
pub mod ui_config;
pub mod connectors;
pub mod bindings;
pub mod departments;
pub mod state;
pub mod cmd;
pub mod snapshot;
pub mod process;
pub mod binding;

use connection::ConnectionState;
use chat::ChatState;
use dialogs::DialogState;
use drag::DragState;
use commands::CommandState;
use ui_config::UiConfig;
use connectors::ConnectorManager;
use bindings::BindingManager;

pub use state::CommandLogEntry;

pub struct PolygramApp {
    pub canvas: Canvas,
    pub connection: ConnectionState,
    pub commands: CommandState,
    pub dialogs: DialogState,
    pub drag: DragState,
    pub right_click: Option<(egui::Pos2, Option<String>)>,
    /// Выделенные объекты (ID) — grupповое выделение через Selection Box и Ctrl+клик.
    pub selected_objects: Vec<String>,
    /// Менеджер API-привязок показателей.
    pub bindings: BindingManager,
    pub show_mcp_panel: bool,
    pub show_sidebar: bool,
    pub show_command_log_panel: bool,
    pub show_frontend_log_panel: bool,
    pub show_chat_panel: bool,
    pub show_connectors_panel: bool,
    pub show_grid: bool,
    pub show_cursor_coords: bool,
    pub last_status: String,
    pub chat: ChatState,
    /// Экспорт/импорт отдела: флаги запроса открытия диалога файла.
    pub export_requested: bool,
    pub import_requested: bool,
    pub export_department_requested: bool,
    pub import_department_requested: bool,
    /// Менеджер коннекторов: кэш файлов connectors/ и статус подключения.
    pub connectors: ConnectorManager,
}

impl PolygramApp {
    pub fn new(snapshot_rx: mpsc::Receiver<Arc<Published>>) -> Self {
        let canvas = Canvas::new();
        let api_client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        let api_base = "http://127.0.0.1:8080".to_owned();
        let mut chat = ChatState::default();
        chat.chat_rx = ChatState::start_polling(api_client.clone(), api_base.clone());
        let ui_config = UiConfig::load();
        Self {
            canvas,
            connection: ConnectionState::new(api_client, api_base, snapshot_rx),
            commands: CommandState::new(),
            dialogs: DialogState::new(),
            drag: DragState::new(),
            right_click: None,
            selected_objects: Vec::new(),
            bindings: BindingManager::new(),
            show_mcp_panel: false,
            show_sidebar: ui_config.show_sidebar,
            show_command_log_panel: ui_config.show_command_log_panel,
            show_frontend_log_panel: ui_config.show_frontend_log_panel,
            show_chat_panel: ui_config.show_chat_panel,
            show_connectors_panel: ui_config.show_connectors_panel,
            show_grid: ui_config.show_grid,
            show_cursor_coords: ui_config.show_cursor_coords,
            last_status: "API сервер не подключён".to_owned(),
            chat,
            export_requested: false,
            import_requested: false,
            export_department_requested: false,
            import_department_requested: false,
            connectors: ConnectorManager::new(),
        }
    }
}

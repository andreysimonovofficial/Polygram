use polygram_canvas::{Canvas, GridMode};
use egui::Color32;
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
pub mod settings;
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
    pub grid_mode: GridMode,
    pub grid_color: [u8; 3],
    pub show_cursor_coords: bool,
    /// Окно настроек темы открыто.
    pub show_settings: bool,
    /// Полноэкранный экран показателей открыт.
    pub show_metrics_screen: bool,
    /// Объект, для которого открыт экран показателей.
    pub metrics_target_object: Option<String>,
    pub last_status: String,
    pub chat: ChatState,
    /// Экспорт/импорт отдела: флаги запроса открытия диалога файла.
    pub export_requested: bool,
    pub import_requested: bool,
    pub export_department_requested: bool,
    pub import_department_requested: bool,
    /// Экспорт/импорт проекта: флаги запроса открытия диалога файла.
    pub open_project_requested: bool,
    pub save_project_requested: bool,
    pub save_project_as_requested: bool,
    /// Менеджер коннекторов: кэш файлов connectors/ и статус подключения.
    pub connectors: ConnectorManager,
    /// Несохранённые изменения (флаг "грязного" состояния).
    pub dirty: bool,
    /// Последний открытый файл проекта.
    pub last_opened_file: Option<String>,
    /// Подтверждение закрытия с несохранёнными изменениями ожидается.
    pub close_confirmation_pending: bool,
}

impl PolygramApp {
    pub fn new(snapshot_rx: mpsc::Receiver<Arc<Published>>) -> Self {
        let mut canvas = Canvas::new();
        let api_client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        let api_base = "http://127.0.0.1:8080".to_owned();
        let mut chat = ChatState::default();
        chat.chat_rx = ChatState::start_polling(api_client.clone(), api_base.clone());
        let ui_config = UiConfig::load();
        let grid_color = ui_config.grid_color;
        // Применяем сохранённую тему оформления.
        canvas.theme = ui_config.theme.into_theme();
        canvas.theme.grid_color = Color32::from_rgb(grid_color[0], grid_color[1], grid_color[2]);
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
            grid_mode: ui_config.grid_mode,
            grid_color,
            show_cursor_coords: ui_config.show_cursor_coords,
            show_settings: false,
            show_metrics_screen: false,
            metrics_target_object: None,
            last_status: "API сервер не подключён".to_owned(),
            chat,
            export_requested: false,
            import_requested: false,
            export_department_requested: false,
            import_department_requested: false,
            open_project_requested: false,
            save_project_requested: false,
            save_project_as_requested: false,
            connectors: ConnectorManager::new(),
            dirty: false,
            last_opened_file: ui_config.last_opened_file.clone(),
            close_confirmation_pending: false,
        }
    }
}

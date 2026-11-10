use polygram_canvas::Canvas;
use polygram_core::{Command, Published};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

pub mod connection;
pub mod chat;
pub mod dialogs;
pub mod drag;
pub mod commands;

use connection::ConnectionState;
use connection::ConnectionResult;
use chat::ChatState;
use dialogs::DialogState;
use drag::DragState;
use commands::CommandState;

pub struct CommandLogEntry {
    pub cmd: String,
    pub result: String,
}

#[derive(Default, Serialize, Deserialize)]
pub struct UiConfig {
    pub show_sidebar: bool,
    pub show_command_log_panel: bool,
    pub show_chat_panel: bool,
    pub show_connectors_panel: bool,
    pub show_grid: bool,
    pub show_cursor_coords: bool,
}

impl UiConfig {
    pub fn path() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
            .join("polygram_ui_config.json")
    }

    pub fn load() -> Self {
        let path = Self::path();
        match std::fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) {
        let path = Self::path();
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, json);
        }
    }
}

pub struct PolygramApp {
    pub canvas: Canvas,
    pub connection: ConnectionState,
    pub commands: CommandState,
    pub dialogs: DialogState,
    pub drag: DragState,
    pub right_click: Option<(egui::Pos2, Option<String>)>,
    /// Выделенные объекты (ID) — grupповое выделение через Selection Box и Ctrl+клик.
    pub selected_objects: Vec<String>,
    /// Статусы API-привязок (ключ: "{object_id}:{entry_id}").
    pub api_bindings: HashMap<String, BindingStatusInfo>,
    /// Тикер для периодического обновления статусов привязок (каждые ~100 кадров ≈ 2 сек).
    pub api_bindings_tick: u32,
    pub show_mcp_panel: bool,
    pub show_sidebar: bool,
    pub show_command_log_panel: bool,
    pub show_chat_panel: bool,
    pub show_connectors_panel: bool,
    pub show_grid: bool,
    pub show_cursor_coords: bool,
    pub last_status: String,
    pub chat: ChatState,
    /// Локальный кэш конфигов коннекторов (чтобы URL не исчезал при перерисовке).
    pub connector_configs: HashMap<String, serde_json::Value>,
    /// Статус подключения каждого коннектора.
    pub connector_status: HashMap<String, ConnectorStatusInfo>,
}

/// Статус подключения коннектора.
#[derive(Clone, Debug, Default)]
pub struct ConnectorStatusInfo {
    pub ok: bool,
    pub last_checked: Option<String>,
    pub error: Option<String>,
}

/// Информация о статусе одной API-привязки для отображения в UI.
#[derive(Clone, Debug, Default)]
pub struct BindingStatusInfo {
    pub ok: bool,
    pub connector: String,
    pub cell: String,
    pub last_value: Option<f64>,
    pub last_updated: Option<String>,
    pub error: Option<String>,
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
            api_bindings: HashMap::new(),
            api_bindings_tick: 0,
            show_mcp_panel: false,
            show_sidebar: ui_config.show_sidebar,
            show_command_log_panel: ui_config.show_command_log_panel,
            show_chat_panel: ui_config.show_chat_panel,
            show_connectors_panel: ui_config.show_connectors_panel,
            show_grid: ui_config.show_grid,
            show_cursor_coords: ui_config.show_cursor_coords,
            last_status: "API сервер не подключён".to_owned(),
            chat,
            connector_configs: Self::load_connector_configs(),
            connector_status: HashMap::new(),
        }
    }

    pub fn ui_config_save(&self) {
        UiConfig {
            show_sidebar: self.show_sidebar,
            show_command_log_panel: self.show_command_log_panel,
            show_chat_panel: self.show_chat_panel,
            show_connectors_panel: self.show_connectors_panel,
            show_grid: self.show_grid,
            show_cursor_coords: self.show_cursor_coords,
        }
        .save();
    }

    pub fn poll_chat(&mut self) {
        self.chat.poll();
    }

    pub fn add_connection_log(&mut self, ok: bool, message: &str) {
        self.connection.add_connection_log(ok, message);
    }

    pub fn apply_cmd(&mut self, cmd: Command) {
        let cmd_str = serde_json::to_string(&cmd).unwrap_or_default();
        let api_base = self.connection.api_base.clone();
        let client = self.connection.api_client.clone();

        std::thread::spawn(move || {
            let result = client.post(format!("{api_base}/api/commands")).json(&cmd).send();

            match result {
                Ok(resp) => {
                    let status = resp.status();
                    let body: Value = resp.json().unwrap_or_default();
                    let ok = body.get("ok").and_then(|v| v.as_bool()).unwrap_or(status.is_success());
                    let result_str = if ok {
                        format!("OK (HTTP {status})")
                    } else {
                        let error = body.get("error").and_then(|v| v.as_str()).unwrap_or("неизвестная ошибка");
                        format!("Ошибка HTTP {status}: {error}")
                    };
                    eprintln!("Команда {cmd_str}: {result_str}");
                }
                Err(e) => {
                    eprintln!("Не удалось отправить команду в API: {e}");
                }
            }
        });

        if self.commands.command_log.len() > 200 {
            self.commands.command_log.remove(0);
        }
    }

    /// Отправить пакет команд одной queries в порядке следования (атомично на сервере).
    pub fn apply_batch(&mut self, cmds: Vec<Command>) {
        let cmd_str = serde_json::to_string(&cmds).unwrap_or_default();
        let api_base = self.connection.api_base.clone();
        let client = self.connection.api_client.clone();

        std::thread::spawn(move || {
            let result = client
                .post(format!("{api_base}/api/commands/batch"))
                .json(&cmds)
                .send();

            match result {
                Ok(resp) => {
                    let status = resp.status();
                    let body: Value = resp.json().unwrap_or_default();
                    let ok = body.get("ok").and_then(|v| v.as_bool()).unwrap_or(status.is_success());
                    let result_str = if ok {
                        format!("OK (HTTP {status})")
                    } else {
                        let error = body.get("error").and_then(|v| v.as_str()).unwrap_or("неизвестная ошибка");
                        format!("Ошибка HTTP {status}: {error}")
                    };
                    eprintln!("Команды {cmd_str}: {result_str}");
                }
                Err(e) => {
                    eprintln!("Не удалось отправить команды в API: {e}");
                }
            }
        });
    }

    pub fn poll_snapshot(&mut self) {
        // Обработка pending_move: если сервер не применил MoveObject (CellOccupied из-за буфера 2 кванта),
        // откатываем клиентское предсказание после короткого таймаута.
        if let Some((ref obj_id, rev_at_move)) = self.drag.pending_move {
            if self.connection.last_rev > rev_at_move {
                // Команда применена сервером — позиция актуальна в снимке.
                self.drag.predicted_positions.remove(obj_id);
                self.drag.pending_move = None;
                self.drag.pending_move_age = 0;
            } else {
                // Команда, вероятно, отклонена — увеличиваем возраст.
                self.drag.pending_move_age += 1;
                if self.drag.pending_move_age > 5 {
                    self.drag.predicted_positions.remove(obj_id);
                    self.drag.pending_move = None;
                    self.drag.pending_move_age = 0;
                }
            }
        }

        while let Ok(new_snapshot) = self.connection.snapshot_rx.try_recv() {
            let new_rev = new_snapshot.rev;
            if new_rev != self.connection.last_rev {
                self.connection.last_rev = new_rev;
                self.connection.snapshot = Some(new_snapshot);
                self.connection.api_connected = true;
                self.connection.api_error = None;
                self.last_status = format!("Ревизия {}", self.connection.last_rev);
                self.add_connection_log(true, &format!("Снимок получен, ревизия {}", new_rev));
                if let Some((ref obj_id, rev_at_move)) = self.drag.pending_move {
                    if new_rev > rev_at_move {
                        self.drag.predicted_positions.remove(obj_id);
                        self.drag.pending_move = None;
                        self.drag.pending_move_age = 0;
                    }
                }
            }
        }
    }

    pub fn geometry_with_predictions(&self, published: &Published) -> polygram_core::geometry::Geometry {
        let mut geometry = published.geometry.clone();
        for obj in &mut geometry.objects {
            if let Some((x, y)) = self.drag.predicted_positions.get(&obj.id) {
                obj.rect.x = *x;
                obj.rect.y = *y;
            }
        }
        geometry
    }

    pub fn try_connect_once(&mut self) {
        match self.connection.try_connect_once() {
            ConnectionResult::Connected { rev } => {
                self.last_status = format!("Ревизия {}", rev);
                self.add_connection_log(true, &format!("Подключено к API, ревизия {}", rev));
            }
            ConnectionResult::Failed { error } => {
                self.last_status = format!("API недоступен: {error}");
                self.add_connection_log(false, &format!("Ошибка подключения: {error}"));
            }
        }
    }

    pub fn start_snapshot_polling(
        api_client: Client,
        api_base: String,
        tx: mpsc::Sender<Arc<Published>>,
    ) {
        std::thread::spawn(move || loop {
            let result = (|| -> Result<Arc<Published>, Box<dyn std::error::Error + Send + Sync>> {
                let resp = api_client.get(format!("{api_base}/api/snapshot")).send()?;
                if !resp.status().is_success() {
                    return Err(format!("HTTP {}", resp.status()).into());
                }
                let published: Published = resp.json()?;
                Ok(Arc::new(published))
            })();

            match result {
                Ok(published) => {
                    let _ = tx.send(published);
                }
                Err(e) => {
                    eprintln!("Ошибка опроса снимка API: {e}");
                }
            }
            std::thread::sleep(Duration::from_millis(500));
        });
    }

    fn exe_dir() -> std::path::PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
    }

    pub fn api_exe_path() -> std::path::PathBuf {
        Self::exe_dir().join("polygram-api.exe")
    }

    pub fn mcp_exe_path() -> std::path::PathBuf {
        Self::exe_dir().join("polygram-mcp.exe")
    }

    pub fn api_process_running(&mut self) -> bool {
        self.connection.api_process_running()
    }

    pub fn start_api_process(&mut self) {
        match self.connection.start_api_process() {
            Ok(msg) => { self.connection.add_connection_log(true, &msg); }
            Err(msg) => { self.connection.add_connection_log(false, &msg); }
        }
    }

    pub fn stop_api_process(&mut self) {
        if self.connection.stop_api_process() {
            self.connection.add_connection_log(true, "polygram-api остановлен");
        }
    }

    pub fn restart_api_process(&mut self) {
        self.stop_api_process();
        self.start_api_process();
    }

    pub fn mcp_process_running(&mut self) -> bool {
        self.connection.mcp_process_running()
    }

    pub fn start_mcp_process(&mut self) {
        match self.connection.start_mcp_process() {
            Ok(msg) => { self.connection.add_connection_log(true, &msg); }
            Err(msg) => { self.connection.add_connection_log(false, &msg); }
        }
    }

    pub fn stop_mcp_process(&mut self) {
        if self.connection.stop_mcp_process() {
            self.connection.add_connection_log(true, "polygram-mcp остановлен");
        }
    }

    pub fn restart_mcp_process(&mut self) {
        self.stop_mcp_process();
        self.start_mcp_process();
    }

    // ----- API-привязки показателей -----

    pub fn post_binding(&mut self, object_id: String, entry_id: String, connector: String, cell: String, interval_secs: u64) {
        let api_base = self.connection.api_base.clone();
        let client = self.connection.api_client.clone();
        std::thread::spawn(move || {
            let result = client
                .post(format!("{api_base}/api/bindings"))
                .json(&serde_json::json!({
                    "object_id": object_id,
                    "entry_id": entry_id,
                    "connector": connector,
                    "cell": cell,
                    "interval_secs": interval_secs,
                }))
                .send();
            match result {
                Ok(resp) => {
                    if resp.status().is_success() {
                        eprintln!("API-привязка создана: {object_id}:{entry_id} -> {connector}:{cell}");
                    } else {
                        eprintln!("Ошибка создания API-привязки: HTTP {}", resp.status());
                    }
                }
                Err(e) => eprintln!("Не удалось создать API-привязку: {e}"),
            }
        });
    }

    pub fn delete_binding(&mut self, object_id: String, entry_id: String) {
        let id = format!("{object_id}:{entry_id}");
        let api_base = self.connection.api_base.clone();
        let client = self.connection.api_client.clone();
        std::thread::spawn(move || {
            let result = client.delete(format!("{api_base}/api/bindings/{id}")).send();
            match result {
                Ok(resp) => {
                    if resp.status().is_success() {
                        eprintln!("API-привязка удалена: {id}");
                    } else {
                        eprintln!("Ошибка удаления API-привязки: HTTP {}", resp.status());
                    }
                }
                Err(e) => eprintln!("Не удалось удалить API-привязку: {e}"),
            }
        });
    }

    pub fn get_connectors(&self) -> Vec<String> {
        let api_base = self.connection.api_base.clone();
        let client = self.connection.api_client.clone();
        match client.get(format!("{api_base}/api/connectors")).send() {
            Ok(resp) => {
                let body: serde_json::Value = resp.json().unwrap_or_default();
                body.get("connectors")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|c| c.get("id").and_then(|v| v.as_str()).map(|s| s.to_owned()))
                            .collect()
                    })
                    .unwrap_or_default()
            }
            Err(_) => Vec::new(),
        }
    }

    pub fn get_binding_statuses(&self) -> serde_json::Value {
        let api_base = self.connection.api_base.clone();
        let client = self.connection.api_client.clone();
        match client.get(format!("{api_base}/api/bindings/status")).send() {
            Ok(resp) => resp.json().unwrap_or_default(),
            Err(_) => serde_json::json!({}),
        }
    }

    /// Обновить кэш статусов API-привязок из сервера.
    pub fn refresh_api_bindings(&mut self) {
        let statuses = self.get_binding_statuses();
        let statuses_map = statuses.get("statuses").and_then(|v| v.as_object());
        if let Some(map) = statuses_map {
            for (id, status) in map {
                let ok = status.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
                let last_value = status.get("last_value").and_then(|v| v.as_f64());
                let last_updated = status.get("last_updated").and_then(|v| v.as_str()).map(|s| s.to_owned());
                let error = status.get("error").and_then(|v| v.as_str()).map(|s| s.to_owned());
                // connector/cell не приходят в статусе, оставим пустые
                self.api_bindings.insert(id.clone(), BindingStatusInfo {
                    ok,
                    connector: String::new(),
                    cell: String::new(),
                    last_value,
                    last_updated,
                    error,
                });
            }
        }
    }

    /// Нарисовать иконки API-привязок на строках показателей холста.
    pub fn paint_api_binding_icons(
        &self,
        painter: &egui::Painter,
    ) {
        if self.api_bindings.is_empty() {
            return;
        }
        for (hotspot, _dir, entry_name, rect) in &self.canvas.drag_metric_hotspots {
            let binding_id = format!("{hotspot}:{entry_name}");
            let info = self.api_bindings.get(&binding_id);
            if info.is_none() {
                continue;
            }
            let info = info.unwrap();
            // Рисуем маленькую иконку справа от строки показателя.
            let icon_x = rect.max.x - 14.0;
            let icon_y = rect.center().y;
            let icon_color = if info.ok {
                egui::Color32::from_rgb(0, 200, 80)
            } else {
                egui::Color32::from_rgb(240, 80, 80)
            };
            let icon_text = if info.ok { "🔗" } else { "⚠" };
            painter.text(
                egui::Pos2::new(icon_x, icon_y),
                egui::Align2::CENTER_CENTER,
                icon_text,
                egui::FontId::new(12.0, egui::FontFamily::Proportional),
                icon_color,
            );
        }
    }

    /// Получить конфиг коннектора по ID (из локального кэша).
    pub fn get_connector_config(&self, connector_id: &str) -> Option<serde_json::Value> {
        self.connector_configs.get(connector_id).cloned()
    }

    /// Сохранить конфиг коннектора по ID (локально + на сервер + в файл).
    pub fn set_connector_config(&mut self, connector_id: &str, config: serde_json::Value) {
        self.connector_configs
            .insert(connector_id.to_owned(), config.clone());
        let _ = self.save_connector_configs();
        let api_base = self.connection.api_base.clone();
        let client = self.connection.api_client.clone();
        let id = connector_id.to_owned();
        std::thread::spawn(move || {
            let _ = client
                .post(format!("{api_base}/api/connectors/{id}/config"))
                .json(&config)
                .send();
        });
    }

    /// Получить список метрик Google Analytics.
    pub fn get_ga_metrics(&self) -> Vec<String> {
        vec![
            "ga:sessions".to_owned(),
            "ga:users".to_owned(),
            "ga:pageviews".to_owned(),
            "ga:bounceRate".to_owned(),
            "ga:avgSessionDuration".to_owned(),
            "ga:goalCompletionsAll".to_owned(),
            "ga:transactions".to_owned(),
            "ga:transactionRevenue".to_owned(),
            "ga4:ga:sessions".to_owned(),
            "ga4:ga:users".to_owned(),
            "ga4:ga:screenviews".to_owned(),
            "ga4:ga:conversions".to_owned(),
        ]
    }

    /// Проверить подключение к Apps Script URL (для Google Таблиц).
    pub fn check_connector_status(&mut self, connector_id: &str) {
        let config = match self.connector_configs.get(connector_id) {
            Some(c) => c.clone(),
            None => return,
        };
        let url = match connector_id {
            "google-sheets" => config
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned(),
            _ => return,
        };

        if url.is_empty() || !url.starts_with("http") {
            self.connector_status.insert(
                connector_id.to_owned(),
                ConnectorStatusInfo {
                    ok: false,
                    last_checked: Some(chrono::Local::now().format("%H:%M:%S").to_string()),
                    error: Some("URL не задан".to_owned()),
                },
            );
            return;
        }

        let api_client = self.connection.api_client.clone();
        let id = connector_id.to_owned();
        std::thread::spawn(move || {
            let result = api_client.get(&url).query(&[("cell", "status")]).send();
            let ok = result.as_ref().map_or(false, |resp| resp.status().is_success());
            let error = result.err().map(|e| e.to_string());
            let _ = ok;
            let _ = error;
            let _ = id;
        });

        self.connector_status.insert(
            connector_id.to_owned(),
            ConnectorStatusInfo {
                ok: false,
                last_checked: Some(chrono::Local::now().format("%H:%M:%S").to_string()),
                error: Some("проверка...".to_owned()),
            },
        );
    }

    // ----- Персистентность конфигов коннекторов -----

    fn connector_configs_path() -> std::path::PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
            .join("polygram_connector_configs.json")
    }

    fn load_connector_configs() -> HashMap<String, serde_json::Value> {
        let path = Self::connector_configs_path();
        match std::fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => HashMap::new(),
        }
    }

    fn save_connector_configs(&self) -> std::io::Result<()> {
        let path = Self::connector_configs_path();
        let json = serde_json::to_string_pretty(&self.connector_configs)?;
        std::fs::write(path, json)
    }
}

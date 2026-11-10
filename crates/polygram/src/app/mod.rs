use polygram_canvas::Canvas;
use polygram_core::{layout::Rect, types::NodeKind, Command, Published};
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
    /// Экспорт/импорт отдела: флаги запроса открытия диалога файла.
    pub export_requested: bool,
    pub import_requested: bool,
    pub export_department_requested: bool,
    pub import_department_requested: bool,
    /// Локальный кэш файлов коннекторов из папки connectors/.
    pub connectors: HashMap<String, ConnectorFile>,
    /// Статус подключения каждого коннектора.
    pub connector_status: HashMap<String, ConnectorStatusInfo>,
}

/// Описание коннектора из файла connectors/{id}.json.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ConnectorFile {
    pub id: String,
    pub name: String,
    pub placeholder: String,
    #[serde(default)]
    pub config: serde_json::Value,
    #[serde(default)]
    pub cache: serde_json::Value,
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
            connectors: Self::load_connectors(),
            connector_status: HashMap::new(),
            export_requested: false,
            import_requested: false,
            export_department_requested: false,
            import_department_requested: false,
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
                // Команда ещё не подтверждена сервером — сохраняем клиентскую
                // позицию («на месте отпускания»), чтобы объект не перепрыгивал
                // назад к стартовой точке, пока команда не дойдёт до сервера.
                self.drag.pending_move_age += 1;
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
                        self.drag.drag_department = None;
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
        // Растянуть отдел (и родительские отделы по цепочке) под перетаскиваемый
        // объект «на лету», пока перетаскивание ещё не подтверждено сервером.
        if let Some(dept_id) = &self.drag.drag_department {
            Self::stretch_department_for_drag(&mut geometry, dept_id);
        }
        geometry
    }

/// Растянуть рамку отдела под содержащийся в нём перетаскиваемый объект «на лету».
/// Членство определяется пересечением с текущей рамкой отдела: объект, попадающий
/// под курсор при старте перетаскивания, считается ребёнком отдела. Тянет
/// рекурсивно родительские отделы по цепочке вверх.
fn stretch_department_for_drag(geometry: &mut polygram_core::geometry::Geometry, dept_id: &str) {
    let mut current = dept_id.to_owned();
    let mut visited: Vec<String> = Vec::new();
    for _ in 0..8 {
        if visited.contains(&current) {
            break;
        }
        visited.push(current.clone());
        let dept_idx = match geometry.objects.iter().position(|o| o.id == current) {
            Some(i) => i,
            None => break,
        };
        let dept = &geometry.objects[dept_idx];
        let mut min_x = dept.rect.x;
        let mut min_y = dept.rect.y;
        let mut max_x = dept.rect.x + dept.rect.w;
        let mut max_y = dept.rect.y + dept.rect.h;
        for obj in &geometry.objects {
            if obj.id == current {
                continue;
            }
            // Прямой ребёнок отдела по иерархии parent — только такие объекты
            // участвуют в построении рамки отдела.
            if obj.parent.as_ref() != Some(&current) {
                continue;
            }
            min_x = min_x.min(obj.rect.x);
            min_y = min_y.min(obj.rect.y);
            max_x = max_x.max(obj.rect.x + obj.rect.w);
            max_y = max_y.max(obj.rect.y + obj.rect.h);
        }
        let new_x = (min_x - 1).max(0);
        let new_y = (min_y - 2).max(0);
        let new_w = (max_x + 1) - new_x;
        let new_h = (max_y + 1) - new_y;
        let new_rect = Rect::new(new_x, new_y, new_w, new_h);
        if new_rect == dept.rect {
            break;
        }
        geometry.objects[dept_idx].rect = new_rect;
        // Переходим к родительскому отделу через поле parent.
        current = match geometry.objects.iter().find(|o| o.id == current)
            .and_then(|o| o.parent.clone())
        {
            Some(p) => p,
            None => break,
        };
    }
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
        // Источник истины — файлы в папке connectors/. Пересканируем при каждом вызове,
        // чтобы только что положительный файл коннектора подхватился без перезапуска.
        let mut ids: Vec<String> = Vec::new();
        let dir = Self::connectors_dir();
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "json") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        ids.push(stem.to_owned());
                    }
                }
            }
        }
        ids.sort();
        ids
    }

    /// Сгруппировать выделенные объекты в новый отдел.
    /// Вычисляет ограничивающий прямоугольник выделения, создаёт отдел,
    /// размещает его вокруг объектов и переводит каждого выбранного объекта в отдел.
    pub fn group_selected_into_department(&mut self, published: &polygram_core::Published) {
        if self.selected_objects.len() < 2 {
            return;
        }
        let geometry = &published.geometry;
        let mut selected_ids: Vec<String> = Vec::new();
        for id in &self.selected_objects {
            if geometry.objects.iter().any(|o| o.id == *id) {
                selected_ids.push(id.clone());
            }
        }
        if selected_ids.is_empty() {
            return;
        }

        // Предсказуемый ID отдела, чтобы сослаться на него в SetObjectParent.
        let dept_id = format!("dept_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0));

        let mut batch: Vec<Command> = Vec::new();
        // Создаём отдел без явных координат (авторазмещение вне зоны детей),
        // затем SetObjectParent вызовет expand_department_to_fit, который
        // переместит отдел так, чтобы он охватывал всех детей.
        batch.push(Command::AddObject {
            id: Some(dept_id.clone()),
            name: "Отдел".to_owned(),
            kind: polygram_core::types::NodeKind::Department,
            at: None,
            parent: None,
            author: None,
        });
        // Переводим каждый выбранный объект в отдел.
        for obj_id in &selected_ids {
            batch.push(Command::SetObjectParent {
                object: obj_id.clone(),
                parent: Some(dept_id.clone()),
            });
        }

        self.apply_batch(batch);
        // Очищаем выделение.
        self.selected_objects.clear();
    }

    pub fn get_binding_statuses(&self) -> serde_json::Value {
        let api_base = self.connection.api_base.clone();
        let client = self.connection.api_client.clone();
        match client.get(format!("{api_base}/api/bindings/status")).send() {
            Ok(resp) => resp.json().unwrap_or_default(),
            Err(_) => serde_json::json!({}),
        }
    }

    /// Получить список всех API-привязок с сервера.
    pub fn get_bindings(&self) -> Vec<serde_json::Value> {
        let api_base = self.connection.api_base.clone();
        let client = self.connection.api_client.clone();
        match client.get(format!("{api_base}/api/bindings")).send() {
            Ok(resp) => {
                let body: serde_json::Value = resp.json().unwrap_or_default();
                body.get("bindings")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default()
            }
            Err(_) => Vec::new(),
        }
    }

    /// Обновить кэш статусов API-привязок из сервера.
    pub fn refresh_api_bindings(&mut self) {
        let statuses = self.get_binding_statuses();
        let bindings = self.get_bindings();
        let statuses_map = statuses.get("statuses").and_then(|v| v.as_object());
        if let Some(map) = statuses_map {
            for (id, status) in map {
                let ok = status.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
                let last_value = status.get("last_value").and_then(|v| v.as_f64());
                let last_updated = status.get("last_updated").and_then(|v| v.as_str()).map(|s| s.to_owned());
                let error = status.get("error").and_then(|v| v.as_str()).map(|s| s.to_owned());
                let (connector, cell) = bindings.iter().find_map(|b| {
                    if b.get("id").and_then(|v| v.as_str()) == Some(id) {
                        Some((
                            b.get("connector").and_then(|v| v.as_str()).unwrap_or("").to_owned(),
                            b.get("cell").and_then(|v| v.as_str()).unwrap_or("").to_owned(),
                        ))
                    } else {
                        None
                    }
                }).unwrap_or_default();
                self.api_bindings.insert(id.clone(), BindingStatusInfo {
                    ok,
                    connector,
                    cell,
                    last_value,
                    last_updated,
                    error,
                });
            }
        }
    }

    /// Получить конфиг коннектора по ID (из файла + кэш + сервер).
    pub fn get_connector_config(&mut self, connector_id: &str) -> Option<serde_json::Value> {
        // Сначала локальный кэш (загружен из файла при старте).
        if let Some(cf) = self.connectors.get(connector_id) {
            return Some(cf.config.clone());
        }
        // Затем пробуем сервер.
        let api_base = self.connection.api_base.clone();
        let client = self.connection.api_client.clone();
        let id = connector_id.to_owned();
        if let Ok(resp) = client.get(format!("{api_base}/api/connectors/{id}/config")).send() {
            if let Ok(body) = resp.json::<serde_json::Value>() {
                if let Some(config) = body.get("config").cloned() {
                    return Some(config);
                }
            }
        }
        None
    }

    /// Обновить все конфиги коннекторов с сервера (поддержка совместимости).
    pub fn refresh_connector_configs(&mut self, _connector_ids: &[String]) {
        // Конфиги загружаются из файлов при старте; сервер синхронизируется при записи.
    }

    /// Сохранить конфиг коннектора по ID (в файл + на сервер + в кэш).
    pub fn set_connector_config(&mut self, connector_id: &str, config: serde_json::Value) {
        let mut cf = self
            .connectors
            .get(connector_id)
            .cloned()
            .unwrap_or(ConnectorFile {
                id: connector_id.to_owned(),
                ..Default::default()
            });
        cf.id = connector_id.to_owned();
        cf.config = config.clone();
        let _ = Self::save_connector_file(&cf);
        self.connectors.insert(connector_id.to_owned(), cf);

        // Синхронизируем на API-сервер (если запущен).
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
        let config = match self.connectors.get(connector_id) {
            Some(c) => c.config.clone(),
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

    // ----- Персистентность коннекторов (папка connectors/) -----

    fn connectors_dir() -> std::path::PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
            .join("connectors")
    }

    fn load_connectors() -> HashMap<String, ConnectorFile> {
        let mut map: HashMap<String, ConnectorFile> = HashMap::new();
        let dir = Self::connectors_dir();
        let _ = std::fs::create_dir_all(&dir);
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "json") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Ok(mut cf) = serde_json::from_str::<ConnectorFile>(&content) {
                            if cf.id.is_empty() {
                                cf.id = path
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or_default()
                                    .to_owned();
                            }
                            map.insert(cf.id.clone(), cf);
                        }
                    }
                }
            }
        }
        map
    }

    fn save_connector_file(cf: &ConnectorFile) -> std::io::Result<()> {
        let dir = Self::connectors_dir();
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(format!("{}.json", cf.id));
        let json = serde_json::to_string_pretty(cf)?;
        std::fs::write(path, json)
    }
}

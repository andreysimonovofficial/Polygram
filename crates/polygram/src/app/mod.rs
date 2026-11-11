use polygram_canvas::Canvas;
use polygram_core::{layout::Rect, types::NodeKind, Command, Published};
use reqwest::blocking::Client;
use serde_json::Value;
use std::collections::HashMap;
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

use connection::ConnectionState;
use connection::ConnectionResult;
use chat::ChatState;
use dialogs::DialogState;
use drag::DragState;
use commands::CommandState;
use ui_config::UiConfig;
use connectors::ConnectorManager;
use bindings::BindingManager;

pub struct CommandLogEntry {
    pub cmd: String,
    pub result: String,
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
    /// Менеджер API-привязок показателей.
    pub bindings: BindingManager,
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
                        // drag_department не сбрасываем здесь: отдел должен сохранить
                        // новый размер после Drop до следующего перетаскивания.
                        // Размер отдела в published.geometry уже обновлён сервером
                        // (expand_department_to_fit), поэтому при сбросе drag_department
                        // отдел примет актуальный размер без мигания к старому.
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
        // Если нависаем над отделом и есть призрачная копия — временно считаем объект
        // ребёнком отдела в позиции копии, чтобы fit_departments_to_children растянул
        // отдел и все родительские отделы, охватывая копию ещё до события Drop.
        // Реальная иерархия меняется только на сервере по команде SetObjectParent при Drop.
        let dragged_id = self.drag.drag_object.as_deref().unwrap_or("");
        let mut temp_reparent: Option<(String, Option<String>)> = None;
        if let Some(dept_id) = &self.drag.drag_over {
            if geometry.objects.iter().any(|o| o.id == *dept_id && o.kind == NodeKind::Department) {
                if let Some((gx, gy)) = self.drag.drag_ghost {
                    if let Some(obj) = geometry.objects.iter_mut().find(|o| o.id == dragged_id) {
                        temp_reparent = Some((obj.id.clone(), obj.parent.clone()));
                        obj.parent = Some(dept_id.clone());
                        obj.rect.x = gx;
                        obj.rect.y = gy;
                    }
                }
            }
        }
        // Отдел не имеет фиксированного размера: его рамка всегда обтягивает
        // прямых детей (с отступом под шапку и по бокам). Пересобираем перед
        // каждой отрисовкой, чтобы рамка актуально следовала за содержимым.
        Self::fit_departments_to_children(&mut geometry);
        // Вернуть родителя перетаскиваемого объекта: реальная иерархия меняется
        // только на сервере по команде SetObjectParent при Drop.
        if let Some((id, parent)) = temp_reparent {
            if let Some(obj) = geometry.objects.iter_mut().find(|o| o.id == id) {
                obj.parent = parent;
            }
        }
        // Растянуть отдел под перетаскиваемый объект «на лету» (страховка
        // во время активного перетаскивания, до подтверждения сервера).
        if let Some(dept_id) = &self.drag.drag_department {
            Self::stretch_department_for_drag(&mut geometry, dept_id);
        }
        geometry
    }

/// Вычислить rect каждого отдела как обтягивающую рамку его прямых детей.
/// Отдел не имеет фиксированного размера: рамка следует за содержимым.
/// Повторный обход нужен, чтобы вложенные отделы уже имели актуальный размер,
/// когда считается родительский отдел. Пустой отдел сохраняет текущий размер
/// (минимальное место под первого ребёнка).
fn fit_departments_to_children(geometry: &mut polygram_core::geometry::Geometry) {
    let mut children: std::collections::HashMap<String, Vec<usize>> = std::collections::HashMap::new();
    for (i, o) in geometry.objects.iter().enumerate() {
        if let Some(p) = &o.parent {
            children.entry(p.clone()).or_default().push(i);
        }
    }
    for _ in 0..8 {
        let mut changed = false;
        for (dept_id, child_indices) in &children {
            if child_indices.is_empty() {
                continue;
            }
            let dept_idx = match geometry.objects.iter().position(|o| o.id == *dept_id) {
                Some(i) => i,
                None => continue,
            };
            let mut min_x = i32::MAX;
            let mut min_y = i32::MAX;
            let mut max_x = i32::MIN;
            let mut max_y = i32::MIN;
            for &ci in child_indices {
                let c = &geometry.objects[ci];
                min_x = min_x.min(c.rect.x);
                min_y = min_y.min(c.rect.y);
                max_x = max_x.max(c.rect.x + c.rect.w);
                max_y = max_y.max(c.rect.y + c.rect.h);
            }
            let new_x = (min_x - 1).max(0);
            let new_y = (min_y - 2).max(0);
            let new_w = (max_x + 1) - new_x;
            let new_h = (max_y + 1) - new_y;
            let new_rect = Rect::new(new_x, new_y, new_w, new_h);
            let dept = &mut geometry.objects[dept_idx];
            if dept.rect != new_rect {
                dept.rect = new_rect;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
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

    pub fn post_binding(
        &mut self,
        object_id: String,
        entry_id: String,
        connector: String,
        cell: String,
        interval_secs: u64,
    ) {
        self.bindings.post_binding(
            object_id,
            entry_id,
            connector,
            cell,
            interval_secs,
            &self.connection.api_client,
            &self.connection.api_base,
        );
    }

    pub fn delete_binding(&mut self, object_id: String, entry_id: String) {
        self.bindings
            .delete_binding(object_id, entry_id, &self.connection.api_client, &self.connection.api_base);
    }

    pub fn get_binding_statuses(&self) -> Value {
        self.bindings
            .get_binding_statuses(&self.connection.api_client, &self.connection.api_base)
    }

    /// Получить список всех API-привязок с сервера.
    pub fn get_bindings(&self) -> Vec<Value> {
        self.bindings
            .get_bindings(&self.connection.api_client, &self.connection.api_base)
    }

    /// Обновить кэш статусов API-привязок из сервера.
    pub fn refresh_api_bindings(&mut self) {
        self.bindings
            .refresh_api_bindings(&self.connection.api_client, &self.connection.api_base);
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
}

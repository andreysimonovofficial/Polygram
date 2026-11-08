use eframe::egui;
use polygram_canvas::Canvas;
use polygram_core::{Command, Direction, MetricKind, Published};
use reqwest::blocking::Client;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

pub struct PolygramApp {
    pub canvas: Canvas,
    pub show_command_palette: bool,
    pub command_input: String,
    pub last_status: String,
    pub right_click: Option<(egui::Pos2, Option<String>)>,
    pub command_log: Vec<CommandLogEntry>,
    pub rename_target: Option<String>,
    pub rename_input: String,
    pub edit_target: Option<String>,
    pub edit_input: String,
    pub add_metric_target: Option<(String, Direction)>,
    pub add_metric_name: String,
    pub add_metric_kind: MetricKind,
    pub add_metric_value: String,
    pub add_metric_unit: String,
    pub container_menu_target: Option<(String, Direction)>,
    pub container_menu_edits: Vec<(String, String, String)>,
    pub drag_object: Option<String>,
    pub drag_start_screen: Option<egui::Pos2>,
    pub drag_start_cell: Option<(i32, i32)>,
    pub warmup_frames: u32,
    pub show_mcp_panel: bool,
    pub show_sidebar: bool,
    pub api_client: Client,
    pub api_base: String,
    pub api_connected: bool,
    pub snapshot: Option<Arc<Published>>,
    pub last_rev: u64,
    pub snapshot_rx: mpsc::Receiver<Arc<Published>>,
    /// Последняя ошибка подключения к API.
    pub api_error: Option<String>,
    /// Лог попыток подключения (последние 50 записей).
    pub connection_log: Vec<ConnectionLogEntry>,
    /// Таймер периодической попытки переподключения (в кадрах).
    pub reconnect_tick: u32,
    /// Процесс polygram-api (если запущен из GUI).
    pub api_process: Option<std::process::Child>,
    /// Процесс polygram-mcp (если запущен из GUI).
    pub mcp_process: Option<std::process::Child>,
    /// Предсказанные позиции объектов при перетаскивании (client-side prediction).
    /// Ключ — id объекта, значение — клетка (x, y).
    pub predicted_positions: HashMap<String, (i32, i32)>,
    /// Ожидает подтверждения от сервера: (id объекта, ревизия на момент отправки команды).
    pub pending_move: Option<(String, u64)>,
}

pub struct CommandLogEntry {
    pub cmd: String,
    pub result: String,
}

pub struct ConnectionLogEntry {
    pub time: String,
    pub message: String,
    pub ok: bool,
}

impl PolygramApp {
    pub fn new() -> Self {
        let canvas = Canvas::new();
        let api_client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        let api_base = "http://127.0.0.1:8080".to_owned();
        let (snapshot_tx, snapshot_rx) = mpsc::channel();
        let _ = snapshot_tx; // канал используется в main() через start_snapshot_polling
        Self {
            canvas,
            show_command_palette: false,
            command_input: String::new(),
            last_status: "API сервер не подключён".to_owned(),
            right_click: None,
            command_log: Vec::new(),
            rename_target: None,
            rename_input: String::new(),
            edit_target: None,
            edit_input: String::new(),
            add_metric_target: None,
            add_metric_name: String::new(),
            add_metric_kind: MetricKind::Number,
            add_metric_value: String::new(),
            add_metric_unit: String::new(),
            container_menu_target: None,
            container_menu_edits: Vec::new(),
            drag_object: None,
            drag_start_screen: None,
            drag_start_cell: None,
            warmup_frames: 120,
            show_mcp_panel: false,
            show_sidebar: true,
            api_client,
            api_base,
            api_connected: false,
            snapshot: None,
            last_rev: 0,
            snapshot_rx,
            api_error: None,
            connection_log: Vec::new(),
            reconnect_tick: 150,
            api_process: None,
            mcp_process: None,
            predicted_positions: HashMap::new(),
            pending_move: None,
        }
    }

    pub fn add_connection_log(&mut self, ok: bool, message: &str) {
        let time = chrono::Local::now().format("%H:%M:%S").to_string();
        self.connection_log.push(ConnectionLogEntry {
            time: time.clone(),
            message: message.to_owned(),
            ok,
        });
        if self.connection_log.len() > 50 {
            self.connection_log.remove(0);
        }
        // Пишем в файл для отладки.
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("C:\\Dev\\Polygram\\target\\debug\\gui_connection.log")
            .and_then(|mut f| {
                use std::io::Write;
                writeln!(f, "[{}] {} {}", time, if ok { "OK" } else { "ERR" }, message)
            });
    }

    pub fn apply_cmd(&mut self, cmd: Command) {
        let cmd_str = serde_json::to_string(&cmd).unwrap_or_default();
        let api_base = self.api_base.clone();
        let client = self.api_client.clone();

        std::thread::spawn(move || {
            let result = client.post(format!("{api_base}/api/commands")).json(&cmd).send();

            match result {
                Ok(resp) => {
                    let status = resp.status();
                    let body: Value = resp.json().unwrap_or_default();
                    let ok = status.is_success();
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

        if self.command_log.len() > 200 {
            self.command_log.remove(0);
        }
    }

    pub fn poll_snapshot(&mut self) {
        while let Ok(new_snapshot) = self.snapshot_rx.try_recv() {
            let new_rev = new_snapshot.rev;
            if new_rev != self.last_rev {
                self.last_rev = new_rev;
                self.snapshot = Some(new_snapshot);
                self.api_connected = true;
                self.api_error = None;
                self.last_status = format!("Ревизия {}", self.last_rev);
                self.add_connection_log(true, &format!("Снимок получен, ревизия {}", new_rev));
                // Reconciliation: если сервер подтвердил наше перемещение (rev вырос),
                // убираем предсказанную позицию для этого объекта.
                if let Some((ref obj_id, rev_at_move)) = self.pending_move {
                    if new_rev > rev_at_move {
                        self.predicted_positions.remove(obj_id);
                        self.pending_move = None;
                    }
                }
            }
        }
    }

    /// Вернуть геометрию с подменой позиций объектов на предсказанные
    /// (client-side prediction для отзывчивого перетаскивания).
    pub fn geometry_with_predictions(&self, published: &Published) -> polygram_core::geometry::Geometry {
        let mut geometry = published.geometry.clone();
        for obj in &mut geometry.objects {
            if let Some((x, y)) = self.predicted_positions.get(&obj.id) {
                obj.rect.x = *x;
                obj.rect.y = *y;
            }
        }
        geometry
    }

    pub fn try_connect_once(&mut self) {
        let api_base = self.api_base.clone();
        let client = self.api_client.clone();

        let result = (|| -> Result<Arc<Published>, Box<dyn std::error::Error + Send + Sync>> {
            let resp = client.get(format!("{api_base}/api/snapshot")).send()?;
            if !resp.status().is_success() {
                return Err(format!("HTTP {}", resp.status()).into());
            }
            let published: Published = resp.json()?;
            Ok(Arc::new(published))
        })();

        match result {
            Ok(published) => {
                let new_rev = published.rev;
                self.last_rev = new_rev;
                self.snapshot = Some(published);
                self.api_connected = true;
                self.api_error = None;
                self.last_status = format!("Ревизия {}", new_rev);
                self.add_connection_log(true, &format!("Подключено к API, ревизия {}", new_rev));
            }
            Err(e) => {
                self.api_connected = false;
                self.api_error = Some(e.to_string());
                self.last_status = format!("API недоступен: {e}");
                self.add_connection_log(false, &format!("Ошибка подключения: {e}"));
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

    // ── Путь к бинарникам в той же директории, что и текущий exe ──────────
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

    // ── polygram-api управление ───────────────────────────────────────────

    pub fn api_process_running(&mut self) -> bool {
        match &mut self.api_process {
            Some(child) => match child.try_wait() {
                Ok(None) => true,
                Ok(Some(_)) => {
                    self.api_process = None;
                    false
                }
                Err(_) => {
                    self.api_process = None;
                    false
                }
            },
            None => false,
        }
    }

    pub fn start_api_process(&mut self) {
        if self.api_process_running() {
            return;
        }
        match std::process::Command::new(Self::api_exe_path()).spawn() {
            Ok(child) => {
                self.api_process = Some(child);
                self.api_error = None;
                self.add_connection_log(true, "polygram-api запущен");
            }
            Err(e) => {
                self.api_error = Some(format!("Не удалось запустить polygram-api: {e}"));
                self.add_connection_log(false, &format!("Ошибка запуска polygram-api: {e}"));
            }
        }
    }

    pub fn stop_api_process(&mut self) {
        if let Some(mut child) = self.api_process.take() {
            let _ = child.kill();
            self.add_connection_log(true, "polygram-api остановлен");
        }
    }

    pub fn restart_api_process(&mut self) {
        self.stop_api_process();
        self.start_api_process();
    }

    // ── polygram-mcp управление ───────────────────────────────────────────

    pub fn mcp_process_running(&mut self) -> bool {
        match &mut self.mcp_process {
            Some(child) => match child.try_wait() {
                Ok(None) => true,
                Ok(Some(_)) => {
                    self.mcp_process = None;
                    false
                }
                Err(_) => {
                    self.mcp_process = None;
                    false
                }
            },
            None => false,
        }
    }

    pub fn start_mcp_process(&mut self) {
        if self.mcp_process_running() {
            return;
        }
        match std::process::Command::new(Self::mcp_exe_path()).spawn() {
            Ok(child) => {
                self.mcp_process = Some(child);
                self.add_connection_log(true, "polygram-mcp запущен");
            }
            Err(e) => {
                self.api_error = Some(format!("Не удалось запустить polygram-mcp: {e}"));
                self.add_connection_log(false, &format!("Ошибка запуска polygram-mcp: {e}"));
            }
        }
    }

    pub fn stop_mcp_process(&mut self) {
        if let Some(mut child) = self.mcp_process.take() {
            let _ = child.kill();
            self.add_connection_log(true, "polygram-mcp остановлен");
        }
    }

    pub fn restart_mcp_process(&mut self) {
        self.stop_mcp_process();
        self.start_mcp_process();
    }
}

use polygram_core::{Command, types::Direction, EngineHandle};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::{Duration, Instant};

/// Привязка показателя объекта к внешнему источнику.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiBinding {
    pub id: String,
    pub object_id: String,
    pub entry_id: String,
    pub connector: String,
    pub cell: String,
    #[serde(default = "default_interval")]
    pub interval_secs: u64,
}

fn default_interval() -> u64 {
    30
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConnectorInfo {
    pub id: String,
    pub name: String,
    pub placeholder: String,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct BindingStatus {
    pub ok: bool,
    pub last_value: Option<f64>,
    pub last_updated: Option<String>,
    pub error: Option<String>,
}

/// Конфигурация коннектора (хранится глобально на уровне API).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConnectorConfig {
    pub id: String,
    #[serde(default)]
    pub config: serde_json::Value,
}

/// Сервис синхронизации показателей с внешними API.
/// Ведёт фоновый поток опроса привязок и применяет значения через SetValue / SetRequired.
pub struct SyncService {
    bindings: Arc<Mutex<Vec<ApiBinding>>>,
    connector_configs: Arc<Mutex<Vec<ConnectorConfig>>>,
    statuses: Arc<Mutex<HashMap<String, BindingStatus>>>,
    engine_handle: EngineHandle,
    stop: Arc<AtomicBool>,
    worker: Mutex<Option<thread::JoinHandle<()>>>,
}

impl SyncService {
    pub fn new(engine_handle: EngineHandle) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let svc = Self {
            bindings: Arc::new(Mutex::new(Vec::new())),
            connector_configs: Arc::new(Mutex::new(Vec::new())),
            statuses: Arc::new(Mutex::new(HashMap::new())),
            engine_handle,
            stop: Arc::clone(&stop),
            worker: Mutex::new(None),
        };

        let bindings = Arc::clone(&svc.bindings);
        let configs = Arc::clone(&svc.connector_configs);
        let statuses = Arc::clone(&svc.statuses);
        let handle = svc.engine_handle.clone();
        let worker_stop = Arc::clone(&stop);

        let worker = thread::Builder::new()
            .name("polygram-api-sync".to_owned())
            .spawn(move || {
                run_sync_loop(bindings, configs, statuses, handle, worker_stop);
            })
            .expect("sync worker thread must start");

        svc.worker.lock().expect("worker mutex").replace(worker);
        svc
    }

    pub fn list_bindings(&self) -> Vec<ApiBinding> {
        self.bindings.lock().map(|b| b.clone()).unwrap_or_default()
    }

    pub fn add_binding(&self, binding: ApiBinding) {
        if let Ok(mut b) = self.bindings.lock() {
            // Удалить старую привязку на тот же объект+запись, если есть.
            b.retain(|ex| ex.object_id != binding.object_id || ex.entry_id != binding.entry_id);
            b.push(binding.clone());
        }
        let _ = self.statuses.lock().map(|mut s| {
            s.insert(binding.id.clone(), BindingStatus::default());
        });
    }

    pub fn remove_binding(&self, id: &str) {
        if let Ok(mut b) = self.bindings.lock() {
            b.retain(|ex| ex.id != id);
        }
        let _ = self.statuses.lock().map(|mut s| {
            s.remove(id);
        });
    }

    pub fn list_statuses(&self) -> HashMap<String, BindingStatus> {
        self.statuses.lock().map(|s| s.clone()).unwrap_or_default()
    }

    pub fn list_connectors(&self) -> Vec<ConnectorInfo> {
        vec![
            ConnectorInfo {
                id: "google-sheets".to_owned(),
                name: "Google Таблицы".to_owned(),
                placeholder: "https://script.google.com/... или Sheet1!A1".to_owned(),
            },
            ConnectorInfo {
                id: "google-analytics".to_owned(),
                name: "Google Analytics".to_owned(),
                placeholder: "ga4:ga:sessions|2024-01-01|today".to_owned(),
            },
            ConnectorInfo {
                id: "static".to_owned(),
                name: "Константа (тест)".to_owned(),
                placeholder: "42".to_owned(),
            },
        ]
    }

    pub fn set_connector_config(&self, config: ConnectorConfig) {
        if let Ok(mut cfgs) = self.connector_configs.lock() {
            cfgs.retain(|c| c.id != config.id);
            cfgs.push(config);
        }
    }

    pub fn get_connector_config(&self, id: &str) -> Option<serde_json::Value> {
        self.connector_configs
            .lock()
            .ok()
            .and_then(|cfgs| cfgs.iter().find(|c| c.id == id).map(|c| c.config.clone()))
    }
}

impl Drop for SyncService {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.get_mut().expect("worker mutex").take() {
            let _ = worker.join();
        }
    }
}

/// Фоновый цикл опроса привязок.
fn run_sync_loop(
    bindings: Arc<Mutex<Vec<ApiBinding>>>,
    configs: Arc<Mutex<Vec<ConnectorConfig>>>,
    statuses: Arc<Mutex<HashMap<String, BindingStatus>>>,
    engine_handle: EngineHandle,
    stop: Arc<AtomicBool>,
) {
    let mut last_poll: HashMap<String, Instant> = HashMap::new();

    while !stop.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(250));

        let current_bindings = match bindings.lock() {
            Ok(b) => b.clone(),
            Err(_) => continue,
        };
        let current_configs = match configs.lock() {
            Ok(c) => c.clone(),
            Err(_) => continue,
        };

        for binding in &current_bindings {
            let interval = Duration::from_secs(binding.interval_secs.max(1));
            let due = last_poll
                .get(&binding.id)
                .map_or(true, |t| t.elapsed() >= interval);
            if !due {
                continue;
            }
            last_poll.insert(binding.id.clone(), Instant::now());

            let object_id = binding.object_id.clone();
            let entry_id = binding.entry_id.clone();
            let binding_id = binding.id.clone();
            let connector = binding.connector.clone();
            let cell = binding.cell.clone();

            let object_id_for_cmd = object_id.clone();
            let entry_id_for_cmd = entry_id.clone();

            // Определяем направление записи (Запрос → required, Выход → value).
            let direction = engine_handle.query(move |e| {
                let hier = e.hierarchy();
                let object_ix = match hier.find_object(&object_id) {
                    Some(ix) => ix,
                    None => return None,
                };
                let entry_ix = match hier.find_entry(object_ix, &entry_id) {
                    Some(ix) => ix,
                    None => return None,
                };
                Some(hier.object_metric(entry_ix).direction)
            });

            let Some(dir) = direction else {
                let _ = statuses.lock().map(|mut s| {
                    s.insert(
                        binding_id.clone(),
                        BindingStatus {
                            ok: false,
                            error: Some(format!(
                                "запись {} не найдена в объекте {}",
                                entry_id_for_cmd, object_id_for_cmd
                            )),
                            ..Default::default()
                        },
                    );
                });
                continue;
            };

            // Получаем конфиг коннектора.
            let connector_config = current_configs
                .iter()
                .find(|c| c.id == connector)
                .map(|c| c.config.clone());

            // Запрашиваем значение у коннектора.
            let result = fetch_value(&connector, &cell, connector_config.as_ref());

            match result {
                Ok(value) => {
                    let cmd = match dir {
                        Some(Direction::Request) => Command::SetRequired {
                            object: object_id_for_cmd.clone(),
                            entry: entry_id_for_cmd.clone(),
                            required: value,
                        },
                        Some(Direction::Output) => Command::SetValue {
                            object: object_id_for_cmd.clone(),
                            entry: entry_id_for_cmd.clone(),
                            value,
                        },
                        None => {
                            let _ = statuses.lock().map(|mut s| {
                                s.insert(
                                    binding_id.clone(),
                                    BindingStatus {
                                        ok: false,
                                        error: Some("неизвестное направление записи".to_owned()),
                                        ..Default::default()
                                    },
                                );
                            });
                            continue;
                        }
                    };

                    match engine_handle.apply(cmd) {
                        Ok(_) => {
                            let now = chrono::Local::now().format("%H:%M:%S").to_string();
                            let _ = statuses.lock().map(|mut s| {
                                s.insert(
                                    binding_id.clone(),
                                    BindingStatus {
                                        ok: true,
                                        last_value: Some(value),
                                        last_updated: Some(now),
                                        error: None,
                                    },
                                );
                            });
                        }
                        Err(e) => {
                            let _ = statuses.lock().map(|mut s| {
                                s.insert(
                                    binding_id.clone(),
                                    BindingStatus {
                                        ok: false,
                                        error: Some(format!("ошибка команды: {e}")),
                                        ..Default::default()
                                    },
                                );
                            });
                        }
                    }
                }
                Err(err) => {
                    let _ = statuses.lock().map(|mut s| {
                        s.insert(
                            binding_id.clone(),
                            BindingStatus {
                                ok: false,
                                error: Some(err),
                                ..Default::default()
                            },
                        );
                    });
                }
            }
        }
    }
}

/// Запросить значение у коннектора.
fn fetch_value(
    connector: &str,
    cell: &str,
    config: Option<&serde_json::Value>,
) -> Result<f64, String> {
    match connector {
        "static" => cell
            .parse::<f64>()
            .map_err(|_| format!("не удалось разобрать константу: {cell}")),
        "google-sheets" => fetch_google_sheets(cell, config),
        "google-analytics" => fetch_google_analytics(cell, config),
        _ => Err(format!("неизвестный коннектор: {connector}")),
    }
}

/// Google Таблицы: GET на URL Apps Script с параметром cell.
/// Ответ ожидается в формате: {"value": 123.5} или просто число.
fn fetch_google_sheets(cell: &str, config: Option<&serde_json::Value>) -> Result<f64, String> {
    let url = config
        .and_then(|c| c.get("url").and_then(|v| v.as_str()))
        .unwrap_or(cell);

    if url.is_empty() || !url.starts_with("http") {
        return Err("укажите URL Google Apps Script в конфиге коннектора google-sheets".to_owned());
    }

    let client = ureq::AgentBuilder::new()
        .timeout_read(Duration::from_secs(8))
        .timeout_write(Duration::from_secs(8))
        .build();

    let response = client.get(url).query("cell", cell).call();
    let response = response.map_err(|e| format!("ошибка запроса к Google Таблицам: {e}"))?;

    let body: String = response
        .into_string()
        .map_err(|e| format!("ошибка чтения ответа Google Таблиц: {e}"))?;

    // Пробуем распарсить как JSON {"value": ...}
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
        if let Some(v) = json.get("value").and_then(|v| v.as_f64()) {
            return Ok(v);
        }
        if let Some(v) = json.as_f64() {
            return Ok(v);
        }
        if let Some(v) = json.as_bool() {
            return Ok(if v { 1.0 } else { 0.0 });
        }
        if let Some(s) = json.as_str() {
            return s
                .parse::<f64>()
                .map_err(|_| format!("нечисловое значение из Google Таблиц: {s}"));
        }
    }

    body.trim()
        .parse::<f64>()
        .map_err(|_| format!("не удалось разобрать ответ Google Таблиц: {body}"))
}

/// Google Analytics: заглушка (OAuth требует отдельной настройки).
/// Возвращает детерминированное значение на основе хеша cell.
fn fetch_google_analytics(cell: &str, _config: Option<&serde_json::Value>) -> Result<f64, String> {
    if cell.is_empty() {
        return Err("укажите метрику Google Analytics (например ga4:ga:sessions)".to_owned());
    }
    let hash: u32 = cell
        .bytes()
        .fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    let value = (hash % 10_000) as f64 / 100.0;
    Ok(value)
}

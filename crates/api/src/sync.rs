use polygram_core::{Command, types::Direction, EngineHandle};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::{Duration, Instant};

const CONNECTORS_DIR_NAME: &str = "connectors";

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

/// Конфигурация коннектора (хранится в файле connectors/{id}.json).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ConnectorConfig {
    pub id: String,
    #[serde(default)]
    pub config: serde_json::Value,
    #[serde(default)]
    pub cache: serde_json::Value,
}

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

/// Сервис синхронизации показателей с внешними API.
/// Ведёт фоновый поток опроса привязок и применяет значения через SetValue / SetRequired.
pub struct SyncService {
    bindings: Arc<Mutex<Vec<ApiBinding>>>,
    connector_configs: Arc<Mutex<Vec<ConnectorConfig>>>,
    statuses: Arc<Mutex<HashMap<String, BindingStatus>>>,
    engine_handle: EngineHandle,
    stop: Arc<AtomicBool>,
    worker: Mutex<Option<thread::JoinHandle<()>>>,
    connectors_dir: PathBuf,
}

impl SyncService {
    pub fn new(engine_handle: EngineHandle) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let connectors_dir = Self::connectors_dir();
        let _ = fs::create_dir_all(&connectors_dir);

        let mut config_vec: Vec<ConnectorConfig> = Vec::new();
        let _ = Self::ensure_default_connectors(&connectors_dir);
        if let Ok(entries) = fs::read_dir(&connectors_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(mut cf) = serde_json::from_str::<ConnectorFile>(&content) {
                            if cf.id.is_empty() {
                                cf.id = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_owned();
                            }
                            config_vec.push(ConnectorConfig {
                                id: cf.id.clone(),
                                config: cf.config,
                                cache: cf.cache,
                            });
                        }
                    }
                }
            }
        }

        let svc = Self {
            bindings: Arc::new(Mutex::new(Self::load_bindings(&connectors_dir))),
            connector_configs: Arc::new(Mutex::new(config_vec)),
            statuses: Arc::new(Mutex::new(HashMap::new())),
            engine_handle,
            stop: Arc::clone(&stop),
            worker: Mutex::new(None),
            connectors_dir,
        };

        let bindings = Arc::clone(&svc.bindings);
        let configs = Arc::clone(&svc.connector_configs);
        let statuses = Arc::clone(&svc.statuses);
        let handle = svc.engine_handle.clone();
        let worker_stop = Arc::clone(&stop);
        let dir = svc.connectors_dir.clone();

        let worker = thread::Builder::new()
            .name("polygram-api-sync".to_owned())
            .spawn(move || {
                run_sync_loop(bindings, configs, statuses, handle, worker_stop, dir);
            })
            .expect("sync worker thread must start");

        svc.worker.lock().expect("worker mutex").replace(worker);
        svc
    }

    fn connectors_dir() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
            .join(CONNECTORS_DIR_NAME)
    }

    fn bindings_path(dir: &PathBuf) -> PathBuf {
        dir.join("bindings.json")
    }

    fn load_bindings(dir: &PathBuf) -> Vec<ApiBinding> {
        let path = Self::bindings_path(dir);
        match fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str::<Vec<ApiBinding>>(&content).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }

    fn save_bindings(dir: &PathBuf, bindings: &[ApiBinding]) -> std::io::Result<()> {
        let path = Self::bindings_path(dir);
        let json = serde_json::to_string_pretty(bindings)?;
        fs::write(path, json)
    }

    /// Создаёт файлы коннекторов по умолчанию, если папка пуста.
    fn ensure_default_connectors(dir: &PathBuf) -> std::io::Result<()> {
        let count = fs::read_dir(dir)?.count();
        if count > 0 {
            return Ok(());
        }
        let defaults = vec![
            ConnectorFile {
                id: "google-sheets".to_owned(),
                name: "Google Таблицы".to_owned(),
                placeholder: "https://docs.google.com/spreadsheets/d/... или Sheet1!A1".to_owned(),
                config: serde_json::json!({"url": "", "intervalSecs": 30}),
                cache: serde_json::json!({}),
            },
            ConnectorFile {
                id: "google-analytics".to_owned(),
                name: "Google Analytics".to_owned(),
                placeholder: "ga4:ga:sessions|2024-01-01|today".to_owned(),
                config: serde_json::json!({"propertyId": "", "channelId": "MINE", "intervalSecs": 300}),
                cache: serde_json::json!({}),
            },
            ConnectorFile {
                id: "static".to_owned(),
                name: "Константа (тест)".to_owned(),
                placeholder: "42".to_owned(),
                config: serde_json::json!({"intervalSecs": 30}),
                cache: serde_json::json!({}),
            },
        ];
        for cf in &defaults {
            let path = dir.join(format!("{}.json", cf.id));
            let _ = fs::write(&path, serde_json::to_string_pretty(cf).unwrap_or_default());
        }
        Ok(())
    }

    pub fn list_bindings(&self) -> Vec<ApiBinding> {
        self.bindings.lock().map(|b| b.clone()).unwrap_or_default()
    }

    pub fn add_binding(&self, binding: ApiBinding) {
        if let Ok(mut b) = self.bindings.lock() {
            b.retain(|ex| ex.object_id != binding.object_id || ex.entry_id != binding.entry_id);
            b.push(binding.clone());
            let _ = Self::save_bindings(&self.connectors_dir, &b);
        }
        let _ = self.statuses.lock().map(|mut s| {
            s.insert(binding.id.clone(), BindingStatus::default());
        });
    }

    pub fn remove_binding(&self, id: &str) {
        if let Ok(mut b) = self.bindings.lock() {
            b.retain(|ex| ex.id != id);
            let _ = Self::save_bindings(&self.connectors_dir, &b);
        }
        let _ = self.statuses.lock().map(|mut s| {
            s.remove(id);
        });
    }

    pub fn list_statuses(&self) -> HashMap<String, BindingStatus> {
        self.statuses.lock().map(|s| s.clone()).unwrap_or_default()
    }

    pub fn list_connectors(&self) -> Vec<ConnectorInfo> {
        let mut result: Vec<ConnectorInfo> = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.connectors_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(cf) = serde_json::from_str::<ConnectorFile>(&content) {
                            let id = if cf.id.is_empty() {
                                path.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_owned()
                            } else {
                                cf.id
                            };
                            result.push(ConnectorInfo {
                                id,
                                name: cf.name,
                                placeholder: cf.placeholder,
                            });
                            continue;
                        }
                    }
                    let id = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_owned();
                    result.push(ConnectorInfo {
                        id: id.clone(),
                        name: id.clone(),
                        placeholder: String::new(),
                    });
                }
            }
        }
        if result.is_empty() {
            result.push(ConnectorInfo {
                id: "static".to_owned(),
                name: "Константа (тест)".to_owned(),
                placeholder: "42".to_owned(),
            });
        }
        result
    }

    pub fn set_connector_config(&self, config: ConnectorConfig) {
        // Пишем в файл connectors/{id}.json, сохраняя существующий кэш.
        let path = self.connectors_dir.join(format!("{}.json", config.id));
        let mut cf = if let Ok(content) = fs::read_to_string(&path) {
            serde_json::from_str::<ConnectorFile>(&content).unwrap_or(ConnectorFile {
                id: config.id.clone(),
                ..Default::default()
            })
        } else {
            ConnectorFile {
                id: config.id.clone(),
                ..Default::default()
            }
        };
        cf.id = config.id.clone();
        let config_value = config.config.clone();
        cf.config = config_value.clone();
        let _ = fs::write(&path, serde_json::to_string_pretty(&cf).unwrap_or_default());

        // Обновляем in-memory кэш, используемый потоком синхронизации.
        if let Ok(mut cfgs) = self.connector_configs.lock() {
            cfgs.retain(|c| c.id != config.id);
            cfgs.push(ConnectorConfig {
                id: config.id.clone(),
                config: config_value,
                cache: cf.cache,
            });
        }
    }

    pub fn get_connector_config(&self, id: &str) -> Option<serde_json::Value> {
        // Сначала in-memory кэш потока синхронизации.
        if let Ok(cfgs) = self.connector_configs.lock() {
            if let Some(c) = cfgs.iter().find(|c| c.id == id) {
                return Some(c.config.clone());
            }
        }
        // Fallback: читаем файл напрямую.
        let path = self.connectors_dir.join(format!("{}.json", id));
        fs::read_to_string(&path).ok().and_then(|content| {
            serde_json::from_str::<ConnectorFile>(&content).ok().map(|cf| cf.config)
        })
    }

    /// Обновить кэш значения по ячейке в файле коннектора.
    pub fn update_connector_cache(&self, id: &str, cell: &str, value: f64) {
        let path = self.connectors_dir.join(format!("{}.json", id));
        let mut cf = if let Ok(content) = fs::read_to_string(&path) {
            serde_json::from_str::<ConnectorFile>(&content).unwrap_or(ConnectorFile {
                id: id.to_owned(),
                ..Default::default()
            })
        } else {
            ConnectorFile {
                id: id.to_owned(),
                ..Default::default()
            }
        };
        cf.id = id.to_owned();
        let mut cache = cf.cache.as_object().cloned().unwrap_or_default();
        cache.insert(cell.to_owned(), serde_json::Value::from(value));
        cf.cache = serde_json::Value::Object(cache);
        let _ = fs::write(&path, serde_json::to_string_pretty(&cf).unwrap_or_default());
        if let Ok(mut cfgs) = self.connector_configs.lock() {
            if let Some(c) = cfgs.iter_mut().find(|c| c.id == id) {
                c.cache = cf.cache.clone();
            }
        }
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

/// Результат получения одной ячейки коннектора.
struct CellResult {
    value: Option<f64>,
    error: Option<String>,
}

/// Фоновый цикл опроса привязок.
/// Опрос выполняется один раз на коннектор за интервал: собираются все ячейки
/// привязок этого коннектора, делается один вызов к внешнему API, затем
/// значения распределяются по соответствующим записям показателей. Это
/// защищает от превышения квоты (например, Google Sheets Read requests per minute).
fn run_sync_loop(
    bindings: Arc<Mutex<Vec<ApiBinding>>>,
    configs: Arc<Mutex<Vec<ConnectorConfig>>>,
    statuses: Arc<Mutex<HashMap<String, BindingStatus>>>,
    engine_handle: EngineHandle,
    stop: Arc<AtomicBool>,
    connectors_dir: PathBuf,
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

        // Группируем привязки по коннектору.
        let mut by_connector: HashMap<String, Vec<ApiBinding>> = HashMap::new();
        for b in &current_bindings {
            by_connector.entry(b.connector.clone()).or_default().push(b.clone());
        }

        for (connector, items) in &by_connector {
            // Интервал опроса коннектора — из его конфига (intervalSecs),
            // по умолчанию 30 сек. Все привязки коннектора опрашиваются разом.
            let interval_secs = current_configs
                .iter()
                .find(|c| c.id == *connector)
                .and_then(|c| c.config.get("intervalSecs").and_then(|v| v.as_u64()))
                .unwrap_or(30)
                .max(1);
            let interval = Duration::from_secs(interval_secs);
            let due = last_poll
                .get(connector)
                .map_or(true, |t| t.elapsed() >= interval);
            if !due {
                continue;
            }
            last_poll.insert(connector.clone(), Instant::now());

            let connector_config = current_configs
                .iter()
                .find(|c| c.id == *connector)
                .map(|c| c.config.clone());

            let cells: Vec<String> = items.iter().map(|b| b.cell.clone()).collect();
            let results = fetch_values(&connector, &cells, connector_config.as_ref());

            for (binding, result) in items.iter().zip(results) {
                apply_binding_result(
                    binding,
                    result,
                    &engine_handle,
                    &statuses,
                    &connectors_dir,
                    connector,
                );
            }
        }
    }
}

/// Применить значение одной ячейки к записи показателя (SetValue / SetRequired)
/// и обновить её статус + кэш в файле коннектора.
fn apply_binding_result(
    binding: &ApiBinding,
    result: Result<f64, String>,
    engine_handle: &EngineHandle,
    statuses: &Arc<Mutex<HashMap<String, BindingStatus>>>,
    connectors_dir: &PathBuf,
    connector: &str,
) {
    let object_id = binding.object_id.clone();
    let entry_id = binding.entry_id.clone();
    let binding_id = binding.id.clone();
    let cell = binding.cell.clone();
    let object_id_cmd = object_id.clone();
    let entry_id_cmd = entry_id.clone();

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
                    error: Some(format!("запись {} не найдена в объекте {}", entry_id_cmd, object_id_cmd)),
                    ..Default::default()
                },
            );
        });
        return;
    };

    match result {
        Ok(value) => {
            let cmd = match dir {
                Some(Direction::Request) => Command::SetRequired {
                    object: object_id_cmd.clone(),
                    entry: entry_id_cmd.clone(),
                    required: value,
                },
                Some(Direction::Output) => Command::SetValue {
                    object: object_id_cmd.clone(),
                    entry: entry_id_cmd.clone(),
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
                    return;
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
                    // Персистим кэш значения в файл коннектора.
                    let cache_path = connectors_dir.join(format!("{connector}.json"));
                    if let Ok(content) = fs::read_to_string(&cache_path) {
                        if let Ok(mut cf) = serde_json::from_str::<ConnectorFile>(&content) {
                            let mut cache = cf
                                .cache
                                .as_object()
                                .cloned()
                                .unwrap_or_default();
                            cache.insert(
                                cell.clone(),
                                serde_json::Value::from(value),
                            );
                            cf.cache = serde_json::Value::Object(cache);
                            let _ = fs::write(
                                &cache_path,
                                serde_json::to_string_pretty(&cf).unwrap_or_default(),
                            );
                        }
                    }
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

/// Запросить значения у коннектора для списка ячеек.
/// Возвращает результаты в том же порядке, что и ячейки.
fn fetch_values(
    connector: &str,
    cells: &[String],
    config: Option<&serde_json::Value>,
) -> Vec<Result<f64, String>> {
    match connector {
        "static" => cells
            .iter()
            .map(|c| {
                c.parse::<f64>()
                    .map_err(|_| format!("не удалось разобрать константу: {c}"))
            })
            .collect(),
        "google-sheets" => fetch_google_sheets_batch(cells, config),
        "google-analytics" => cells
            .iter()
            .map(|c| fetch_google_analytics(c, config))
            .collect(),
        _ => cells
            .iter()
            .map(|_| Err(format!("неизвестный коннектор: {connector}")))
            .collect(),
    }
}

/// Извлечь ID таблицы из URL Google Таблиц.
/// Пример: https://docs.google.com/spreadsheets/d/1abc2def3ghi4jkl5mno6pqr7stu8vwx9yz/edit?gid=0
/// → 1abc2def3ghi4jkl5mno6pqr7stu8vwx9yz
fn extract_spreadsheet_id(url: &str) -> Option<String> {
    let parts: Vec<&str> = url.split("/d/").collect();
    if parts.len() < 2 {
        return None;
    }
    let rest = parts[1];
    let id = rest.split('/').next()?.split('?').next()?;
    if id.is_empty() {
        None
    } else {
        Some(id.to_owned())
    }
}

/// Путь к бинарнику gws.exe.
fn gws_binary_path() -> String {
    // Пробуем найти в PATH, затем по известному ubicación.
    if std::process::Command::new("gws.exe").arg("--help").output().is_ok() {
        return "gws.exe".to_owned();
    }
    r"C:\Users\andre\AppData\Roaming\npm\node_modules\@googleworkspace\cli\bin\gws.exe"
        .to_owned()
}

/// Google Таблицы: читаем значения нескольких ячеек за один вызов gws CLI.
/// Возвращает результаты в том же порядке, что и ячейки.
fn fetch_google_sheets_batch(
    cells: &[String],
    config: Option<&serde_json::Value>,
) -> Vec<Result<f64, String>> {
    let spreadsheet_id = match config
        .and_then(|c| c.get("url").and_then(|v| v.as_str()))
        .and_then(extract_spreadsheet_id)
    {
        Some(id) => id,
        None => {
            return cells
                .iter()
                .map(|_| {
                    Err(
                        "не задан URL Google Таблицы в конфиге коннектора google-sheets. Укажите URL таблицы в сайдбаре «Коннекторы» → Google Таблицы"
                            .to_owned(),
                    )
                })
                .collect();
        }
    };

    let gws_path = gws_binary_path();

    // Параметры запроса к Google Sheets API (несколько диапазонов за один вызов).
    let params = serde_json::json!({
        "spreadsheetId": spreadsheet_id,
        "ranges": cells,
        "fields": "sheets(data(rowData(values(userEnteredValue,formattedValue))))"
    });

    let output = std::process::Command::new("cmd")
        .args([
            "/c",
            &gws_path,
            "sheets", "spreadsheets", "get",
            "--params", &params.to_string(),
            "--format", "json"
        ])
        .output()
        .map_err(|e| {
            let err = format!("не удалось запустить gws.exe ({gws_path}): {e}");
            cells.iter().map(|_| Err(err.clone())).collect::<Vec<_>>()
        });

    let output = match output {
        Ok(o) => o,
        Err(errs) => return errs,
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let err = format!("gws.exe завершился с ошибкой: stdout={stdout} stderr={stderr}");
        return cells.iter().map(|_| Err(err.clone())).collect();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Отладочный вывод в stderr сервера для диагностики.
    eprintln!("[gws debug] stdout={stdout}");
    // Извлекаем JSON-объект из вывода gws (пропуская служебную строку "Using keyring backend").
    let first_brace = match stdout.find('{') {
        Some(p) => p,
        None => {
            let err = format!("не найден JSON в ответе gws.exe. stdout={stdout}");
            return cells.iter().map(|_| Err(err.clone())).collect();
        }
    };
    let last_brace = match stdout.rfind('}') {
        Some(p) => p,
        None => {
            let err = format!("не найден закрывающий скобку JSON в ответе gws.exe. stdout={stdout}");
            return cells.iter().map(|_| Err(err.clone())).collect();
        }
    };
    let json_str = &stdout[first_brace..=last_brace];

    let resp: serde_json::Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(e) => {
            let err = format!("не удалось распарсить ответ gws: {e}");
            return cells.iter().map(|_| Err(err.clone())).collect();
        }
    };

    // data[i] соответствует ranges[i].
    let data_arr = resp
        .get("sheets")
        .and_then(|s| s.as_array())
        .and_then(|arr| arr.first())
        .and_then(|s| s.get("data"))
        .and_then(|d| d.as_array());

    let mut results = Vec::with_capacity(cells.len());
    for (i, cell) in cells.iter().enumerate() {
        let value = data_arr
            .and_then(|d| d.get(i))
            .and_then(|d| d.get("rowData"))
            .and_then(|rd| rd.as_array())
            .and_then(|arr| arr.first())
            .and_then(|rd| rd.get("values"))
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("userEnteredValue"))
            .and_then(|uev| uev.get("numberValue"))
            .and_then(|n| n.as_f64())
            .or_else(|| {
                // Fallback на formattedValue.
                data_arr
                    .and_then(|d| d.get(i))
                    .and_then(|d| d.get("rowData"))
                    .and_then(|rd| rd.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|rd| rd.get("values"))
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|v| v.get("formattedValue"))
                    .and_then(|fv| fv.as_str())
                    .and_then(|s| s.parse::<f64>().ok())
            });
        results.push(value.ok_or_else(|| {
            format!(
                "ячейка {cell} в таблице {spreadsheet_id} пуста или не содержит числовое значение"
            )
        }));
    }
    results
}

/// Google Analytics: заглушка (OAuth требует отдельной настройки).
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

use chrono::{DateTime, NaiveDate, Utc};
use polygram_core::{Command, types::{Direction, MetricKind, MetricValue, ParseHint}, EngineHandle};
use serde::{Deserialize, Serialize};
use serde_json::Value;
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
    pub config: Value,
    #[serde(default)]
    pub cache: Value,
}

/// Конфигурация коннектора (хранится в файле connectors/{id}.json).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ConnectorConfig {
    pub id: String,
    #[serde(default)]
    pub config: Value,
    #[serde(default)]
    pub cache: Value,
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

#[derive(Clone, Debug, Default, Serialize)]
pub struct ConnectorInfo {
    pub id: String,
    pub name: String,
    pub placeholder: String,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct BindingStatus {
    pub ok: bool,
    pub last_value: Option<String>,
    pub last_updated: Option<String>,
    pub error: Option<String>,
}

/// Трейт для источников данных показателей.
pub trait MetricSource: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn supported_kinds(&self) -> Vec<MetricKind>;
    fn fetch(
        &self,
        cell: &str,
        config: Option<&Value>,
        kind: MetricKind,
        parse_hint: &ParseHint,
    ) -> Result<MetricValue, String>;
    fn placeholder(&self) -> &str;
}

/// Список зарегистрированных источников.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SourceType {
    Static,
    GoogleSheets,
    Todoist,
}

impl MetricSource for SourceType {
    fn id(&self) -> &str {
        match self {
            SourceType::Static => "static",
            SourceType::GoogleSheets => "google-sheets",
            SourceType::Todoist => "todoist",
        }
    }
    fn name(&self) -> &str {
        match self {
            SourceType::Static => "Константа (тест)",
            SourceType::GoogleSheets => "Google Таблицы",
            SourceType::Todoist => "Todoist",
        }
    }
    fn supported_kinds(&self) -> Vec<MetricKind> {
        match self {
            SourceType::Static => vec![MetricKind::Number, MetricKind::Text, MetricKind::Binary],
            SourceType::GoogleSheets => {
                vec![MetricKind::Number, MetricKind::Date, MetricKind::Deadline, MetricKind::Text]
            }
            SourceType::Todoist => {
                vec![MetricKind::Deadline, MetricKind::Duration, MetricKind::Text]
            }
        }
    }
    fn fetch(
        &self,
        cell: &str,
        config: Option<&Value>,
        kind: MetricKind,
        parse_hint: &ParseHint,
    ) -> Result<MetricValue, String> {
        match self {
            SourceType::Static => StaticSource.fetch(cell, config, kind, parse_hint),
            SourceType::GoogleSheets => GoogleSheetsSource.fetch(cell, config, kind, parse_hint),
            SourceType::Todoist => TodoistSource.fetch(cell, config, kind, parse_hint),
        }
    }
    fn placeholder(&self) -> &str {
        match self {
            SourceType::Static => "42",
            SourceType::GoogleSheets => "https://docs.google.com/spreadsheets/d/... или Sheet1!A1",
            SourceType::Todoist => "todoist:task:ID:due_date",
        }
    }
}

/// Источник: constantes.
struct StaticSource;
impl MetricSource for StaticSource {
    fn id(&self) -> &str { "static" }
    fn name(&self) -> &str { "Константа (тест)" }
    fn supported_kinds(&self) -> Vec<MetricKind> {
        vec![MetricKind::Number, MetricKind::Text, MetricKind::Binary]
    }
    fn fetch(&self, cell: &str, _config: Option<&Value>, kind: MetricKind, _parse_hint: &ParseHint) -> Result<MetricValue, String> {
        match kind {
            MetricKind::Number => cell
                .parse::<f64>()
                .map(MetricValue::Number)
                .map_err(|_| format!("не удалось разобрать константу: {cell}")),
            MetricKind::Text => Ok(MetricValue::Text(cell.to_owned())),
            MetricKind::Binary => match cell.trim().to_lowercase().as_str() {
                "true" | "1" | "да" | "yes" => Ok(MetricValue::Binary(true)),
                "false" | "0" | "нет" | "no" => Ok(MetricValue::Binary(false)),
                _ => Err(format!("не удалось разобрать булево значение: {cell}")),
            },
            _ => Err(format!("статический источник не поддерживает тип {:?}", kind)),
        }
    }
    fn placeholder(&self) -> &str { "42" }
}

/// Источник: Google Таблицы.
struct GoogleSheetsSource;
impl MetricSource for GoogleSheetsSource {
    fn id(&self) -> &str { "google-sheets" }
    fn name(&self) -> &str { "Google Таблицы" }
    fn supported_kinds(&self) -> Vec<MetricKind> {
        vec![MetricKind::Number, MetricKind::Date, MetricKind::Deadline, MetricKind::Text]
    }
    fn fetch(&self, cell: &str, config: Option<&Value>, kind: MetricKind, parse_hint: &ParseHint) -> Result<MetricValue, String> {
        let spreadsheet_id = match config
            .and_then(|c| c.get("url").and_then(|v| v.as_str()))
            .and_then(extract_spreadsheet_id)
        {
            Some(id) => id,
            None => {
                return Err(
                    "не задан URL Google Таблицы в конфиге коннектора google-sheets. Укажите URL таблицы в сайдбаре «Коннекторы» → Google Таблицы"
                        .to_owned(),
                );
            }
        };

        let gws_path = gws_binary_path();

        let params = serde_json::json!({
            "spreadsheetId": spreadsheet_id,
            "ranges": [cell],
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
            .map_err(|e| format!("не удалось запустить gws.exe ({gws_path}): {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(format!("gws.exe завершился с ошибкой: stdout={stdout} stderr={stderr}"));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let first_brace = match stdout.find('{') {
            Some(p) => p,
            None => return Err(format!("не найден JSON в ответе gws.exe. stdout={stdout}")),
        };
        let last_brace = match stdout.rfind('}') {
            Some(p) => p,
            None => return Err(format!("не найден закрывающий скобку JSON в ответе gws.exe. stdout={stdout}")),
        };
        let json_str = &stdout[first_brace..=last_brace];

        let resp: Value = match serde_json::from_str(json_str) {
            Ok(v) => v,
            Err(e) => return Err(format!("не удалось распарсить ответ gws: {e}")),
        };

        let value = match resp.get("userEnteredValue") {
            Some(v) => v.clone(),
            None => resp
                .get("sheets")
                .and_then(|s| s.as_array())
                .and_then(|arr| arr.first())
                .and_then(|s| s.get("data"))
                .and_then(|d| d.as_array())
                .and_then(|d| d.first())
                .and_then(|d| d.get("rowData"))
                .and_then(|rd| rd.as_array())
                .and_then(|arr| arr.first())
                .and_then(|rd| rd.get("values"))
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.get("formattedValue"))
                .and_then(|fv| fv.as_str())
                .map(|s| Value::String(s.to_owned()))
                .ok_or_else(|| format!("ячейка {cell} в таблице {spreadsheet_id} пуста"))?,
        };

        parse_google_sheets_value(&value, kind, parse_hint, cell, &spreadsheet_id)
    }
    fn placeholder(&self) -> &str { "https://docs.google.com/spreadsheets/d/... или Sheet1!A1" }
}

/// Источник: Todoist (заглушка — требует OAuth).
struct TodoistSource;
impl MetricSource for TodoistSource {
    fn id(&self) -> &str { "todoist" }
    fn name(&self) -> &str { "Todoist" }
    fn supported_kinds(&self) -> Vec<MetricKind> {
        vec![MetricKind::Deadline, MetricKind::Duration, MetricKind::Text]
    }
    fn fetch(&self, cell: &str, _config: Option<&Value>, kind: MetricKind, _parse_hint: &ParseHint) -> Result<MetricValue, String> {
        if cell.is_empty() {
            return Err("укажите ссылку на задачу Todoist (например todoist:task:123:due_date)".to_owned());
        }
        let hash: u32 = cell.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
        match kind {
            MetricKind::Deadline => {
                let days_from_now = (hash % 30) as i64;
                let dt = Utc::now() + chrono::Duration::days(days_from_now);
                Ok(MetricValue::Deadline(dt))
            }
            MetricKind::Duration => {
                let minutes = (hash % 120) as i64;
                Ok(MetricValue::Duration(minutes * 60_000))
            }
            MetricKind::Text => Ok(MetricValue::Text(format!("todoist:{cell}"))),
            _ => Err(format!("Todoist не поддерживает тип {:?}", kind)),
        }
    }
    fn placeholder(&self) -> &str { "todoist:task:ID:due_date" }
}

/// Реестр источников данных.
#[derive(Clone, Default)]
struct SourceRegistry {
    sources: HashMap<String, SourceType>,
}

impl SourceRegistry {
    fn new() -> Self {
        let mut sources: HashMap<String, SourceType> = HashMap::new();
        sources.insert("static".to_owned(), SourceType::Static);
        sources.insert("google-sheets".to_owned(), SourceType::GoogleSheets);
        sources.insert("todoist".to_owned(), SourceType::Todoist);
        Self { sources }
    }

    fn get(&self, id: &str) -> Option<SourceType> {
        self.sources.get(id).copied()
    }
}

/// Сервис синхронизации показателей с внешними API.
pub struct SyncService {
    bindings: Arc<Mutex<Vec<ApiBinding>>>,
    connector_configs: Arc<Mutex<Vec<ConnectorConfig>>>,
    sources: Arc<Mutex<SourceRegistry>>,
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
            sources: Arc::new(Mutex::new(SourceRegistry::new())),
            statuses: Arc::new(Mutex::new(HashMap::new())),
            engine_handle,
            stop: Arc::clone(&stop),
            worker: Mutex::new(None),
            connectors_dir,
        };

        let bindings = Arc::clone(&svc.bindings);
        let configs = Arc::clone(&svc.connector_configs);
        let sources = Arc::clone(&svc.sources);
        let statuses = Arc::clone(&svc.statuses);
        let handle = svc.engine_handle.clone();
        let worker_stop = Arc::clone(&stop);
        let dir = svc.connectors_dir.clone();

        let worker = thread::Builder::new()
            .name("polygram-api-sync".to_owned())
            .spawn(move || {
                run_sync_loop(bindings, configs, sources, statuses, handle, worker_stop, dir);
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
        crate::atomic_save_with_backup(&path, &json)?;
        Ok(())
    }

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
                id: "todoist".to_owned(),
                name: "Todoist".to_owned(),
                placeholder: "todoist:task:ID:due_date".to_owned(),
                config: serde_json::json!({"intervalSecs": 60}),
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
        let json = serde_json::to_string_pretty(&cf).unwrap_or_default();
        let _ = crate::atomic_save_with_backup(&path, &json);

        if let Ok(mut cfgs) = self.connector_configs.lock() {
            cfgs.retain(|c| c.id != config.id);
            cfgs.push(ConnectorConfig {
                id: config.id.clone(),
                config: config_value,
                cache: cf.cache,
            });
        }
    }

    pub fn get_connector_config(&self, id: &str) -> Option<Value> {
        if let Ok(cfgs) = self.connector_configs.lock() {
            if let Some(c) = cfgs.iter().find(|c| c.id == id) {
                return Some(c.config.clone());
            }
        }
        let path = self.connectors_dir.join(format!("{}.json", id));
        fs::read_to_string(&path).ok().and_then(|content| {
            serde_json::from_str::<ConnectorFile>(&content).ok().map(|cf| cf.config)
        })
    }

    pub fn update_connector_cache(&self, id: &str, cell: &str, value: &MetricValue) {
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
        cache.insert(cell.to_owned(), serde_json::to_value(value).unwrap_or(Value::Null));
        cf.cache = serde_json::Value::Object(cache);
        let json = serde_json::to_string_pretty(&cf).unwrap_or_default();
        let _ = crate::atomic_save_with_backup(&path, &json);
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

/// Фоновый цикл опроса привязок.
fn run_sync_loop(
    bindings: Arc<Mutex<Vec<ApiBinding>>>,
    configs: Arc<Mutex<Vec<ConnectorConfig>>>,
    sources: Arc<Mutex<SourceRegistry>>,
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

        let mut by_connector: HashMap<String, Vec<ApiBinding>> = HashMap::new();
        for b in &current_bindings {
            by_connector.entry(b.connector.clone()).or_default().push(b.clone());
        }

        for (connector, items) in &by_connector {
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
            let source = {
                let guard = match sources.lock() {
                    Ok(g) => g,
                    Err(_) => continue,
                };
                guard.get(connector)
            };
            let Some(source) = source else {
                continue;
            };

            for binding in items {
                let object_id = binding.object_id.clone();
                let entry_id = binding.entry_id.clone();
                let binding_id = binding.id.clone();
                let cell = binding.cell.clone();

                let oid = object_id.clone();
                let eid = entry_id.clone();
                let kind_hint = engine_handle.query(move |e| {
                    let hier = e.hierarchy();
                    let object_ix = match hier.find_object(&oid) {
                        Some(ix) => ix,
                        None => return None,
                    };
                    let entry_ix = match hier.find_entry(object_ix, &eid) {
                        Some(ix) => ix,
                        None => return None,
                    };
                    let om = hier.object_metric(entry_ix);
                    let md = hier.metric_def(om.metric_def);
                    let hint = md.api.as_ref().map(|a| a.parse_hint.clone()).unwrap_or_default();
                    Some((md.kind, hint))
                });

                let Some((kind, parse_hint)) = kind_hint.and_then(|x| x) else {
                    let _ = statuses.lock().map(|mut s| {
                        s.insert(
                            binding_id.clone(),
                            BindingStatus {
                                ok: false,
                                error: Some(format!("запись {} не найдена в объекте {}", entry_id, object_id)),
                                ..Default::default()
                            },
                        );
                    });
                    continue;
                };

                let result = source.fetch(&cell, connector_config.as_ref(), kind, &parse_hint);
                apply_binding_result(
                    &ApiBinding {
                        id: binding_id,
                        object_id,
                        entry_id,
                        connector: connector.clone(),
                        cell,
                        interval_secs: binding.interval_secs,
                    },
                    result,
                    &engine_handle,
                    &statuses,
                    &connectors_dir,
                    &connector,
                );
            }
        }
    }
}

/// Применить значение одной ячейки к записи показателя (SetValue / SetRequired)
/// и обновить её статус + кэш в файле коннектора.
fn apply_binding_result(
    binding: &ApiBinding,
    result: Result<MetricValue, String>,
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
                    required: value.clone(),
                },
                Some(Direction::Output) => Command::SetValue {
                    object: object_id_cmd.clone(),
                    entry: entry_id_cmd.clone(),
                    value: value.clone(),
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
                    let value_str = serde_json::to_string(&value).unwrap_or_default();
                    let _ = statuses.lock().map(|mut s| {
                        s.insert(
                            binding_id.clone(),
                            BindingStatus {
                                ok: true,
                                last_value: Some(value_str),
                                last_updated: Some(now),
                                error: None,
                            },
                        );
                    });
                    let cache_path = connectors_dir.join(format!("{connector}.json"));
                    if let Ok(content) = fs::read_to_string(&cache_path) {
                        if let Ok(mut cf) = serde_json::from_str::<ConnectorFile>(&content) {
                            let mut cache = cf.cache.as_object().cloned().unwrap_or_default();
                            cache.insert(
                                cell.clone(),
                                serde_json::to_value(&value).unwrap_or(Value::Null),
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

/// Извлечь ID таблицы из URL Google Таблиц.
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
    if std::process::Command::new("gws.exe").arg("--help").output().is_ok() {
        return "gws.exe".to_owned();
    }
    r"C:\Users\andre\AppData\Roaming\npm\node_modules\@googleworkspace\cli\bin\gws.exe"
        .to_owned()
}

/// Распарсить сырое значение Google Sheets в MetricValue согласно типу показателя.
fn parse_google_sheets_value(
    raw: &Value,
    kind: MetricKind,
    parse_hint: &ParseHint,
    cell: &str,
    spreadsheet_id: &str,
) -> Result<MetricValue, String> {
    match kind {
        MetricKind::Number => {
            if let Some(n) = raw.get("numberValue").and_then(|v| v.as_f64()) {
                return Ok(MetricValue::Number(n));
            }
            if let Some(s) = raw.as_str().and_then(|s| s.parse::<f64>().ok()) {
                return Ok(MetricValue::Number(s));
            }
            Err(format!("ячейка {cell} в таблице {spreadsheet_id} не содержит число"))
        }
        MetricKind::Date | MetricKind::Deadline => {
            if let Some(fmt) = raw.get("formattedValue").and_then(|v| v.as_str()) {
                if let Some(dt) = parse_date_string(fmt, parse_hint) {
                    return match kind {
                        MetricKind::Deadline => Ok(MetricValue::Deadline(dt)),
                        _ => Ok(MetricValue::Date(dt)),
                    };
                }
            }
            if let Some(n) = raw.get("numberValue").and_then(|v| v.as_f64()) {
                if let Some(dt) = serial_date_to_datetime(n) {
                    return match kind {
                        MetricKind::Deadline => Ok(MetricValue::Deadline(dt)),
                        _ => Ok(MetricValue::Date(dt)),
                    };
                }
            }
            Err(format!("ячейка {cell} в таблице {spreadsheet_id} не содержит дату"))
        }
        MetricKind::Text => {
            if let Some(s) = raw.get("formattedValue").and_then(|v| v.as_str()) {
                Ok(MetricValue::Text(s.to_owned()))
            } else if let Some(s) = raw.as_str() {
                Ok(MetricValue::Text(s.to_owned()))
            } else {
                Err(format!("ячейка {cell} в таблице {spreadsheet_id} не содержит текст"))
            }
        }
        _ => Err(format!("Google Таблицы не поддерживают тип {:?}", kind)),
    }
}

/// Распарсить строку даты в DateTime<Utc> согласно parse_hint.
fn parse_date_string(s: &str, parse_hint: &ParseHint) -> Option<DateTime<Utc>> {
    match parse_hint {
        ParseHint::DateFormat(fmt) if fmt == "ISO 8601" || fmt.is_empty() => {
            DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.with_timezone(&Utc))
        }
        ParseHint::DateFormat(fmt) => {
            NaiveDate::parse_from_str(s, fmt)
                .ok()
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .map(|dt| dt.and_utc())
        }
        ParseHint::None => {
            NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .ok()
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .map(|dt| dt.and_utc())
                .or_else(|| DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.with_timezone(&Utc)))
        }
        _ => None,
    }
}

/// Преобразовать серийную дату Google Sheets в DateTime<Utc>.
fn serial_date_to_datetime(serial: f64) -> Option<DateTime<Utc>> {
    let epoch = NaiveDate::from_ymd_opt(1899, 12, 30)?;
    let days = serial as i64;
    let frac = serial.fract();
    let date = epoch.checked_add_signed(chrono::Duration::days(days))?;
    let seconds = (frac * 86400.0).round() as i64;
    let dt = date.and_hms_opt(0, 0, 0)?;
    let dt = dt.checked_add_signed(chrono::Duration::seconds(seconds))?;
    Some(dt.and_utc())
}

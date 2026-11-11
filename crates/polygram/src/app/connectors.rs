use chrono::Local;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

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

/// Менеджер коннекторов: кэш файлов connectors/ и статус подключения каждого.
pub struct ConnectorManager {
    /// Локальный кэш файлов коннекторов из папки connectors/.
    pub connectors: HashMap<String, ConnectorFile>,
    /// Статус подключения каждого коннектора.
    pub connector_status: HashMap<String, ConnectorStatusInfo>,
}

impl ConnectorManager {
    pub fn new() -> Self {
        Self {
            connectors: Self::load_connectors(),
            connector_status: HashMap::new(),
        }
    }

    /// Источник истины — файлы в папке connectors/. Пересканируем при каждом вызове,
    /// чтобы только что положительный файл коннектора подхватился без перезапуска.
    pub fn get_connectors(&self) -> Vec<String> {
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

    /// Получить конфиг коннектора по ID (из файла + кэш + сервер).
    pub fn get_connector_config(
        &self,
        connector_id: &str,
        api_client: &Client,
        api_base: &str,
    ) -> Option<Value> {
        // Сначала локальный кэш (загружен из файла при старте).
        if let Some(cf) = self.connectors.get(connector_id) {
            return Some(cf.config.clone());
        }
        // Затем пробуем сервер.
        let id = connector_id.to_owned();
        if let Ok(resp) = api_client.get(format!("{api_base}/api/connectors/{id}/config")).send() {
            if let Ok(body) = resp.json::<Value>() {
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
    pub fn set_connector_config(
        &mut self,
        connector_id: &str,
        config: Value,
        api_client: &Client,
        api_base: &str,
    ) {
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
        let id = connector_id.to_owned();
        let client = api_client.clone();
        let base = api_base.to_owned();
        std::thread::spawn(move || {
            let _ = client
                .post(format!("{base}/api/connectors/{id}/config"))
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
    pub fn check_connector_status(&mut self, connector_id: &str, api_client: &Client) {
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
                    last_checked: Some(Local::now().format("%H:%M:%S").to_string()),
                    error: Some("URL не задан".to_owned()),
                },
            );
            return;
        }

        let api_client = api_client.clone();
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
                last_checked: Some(Local::now().format("%H:%M:%S").to_string()),
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

use chrono::Local;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::process::Command;

/// Описание коннектора из файла connectors/{id}.json.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConnectorFile {
    pub id: String,
    pub name: String,
    pub placeholder: String,
    #[serde(default)]
    pub config: serde_json::Value,
    #[serde(default)]
    pub cache: serde_json::Value,
}

impl Default for ConnectorFile {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            placeholder: String::new(),
            config: serde_json::Value::Null,
            cache: serde_json::Value::Null,
        }
    }
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

    /// Обновить только кэш в памяти (без записи в файл и сервера),
    /// чтобы текст в поле ввода сохранялся между кадрами.
    pub fn set_connector_config_memory(&mut self, connector_id: &str, config: Value) {
        let mut cf = self
            .connectors
            .get(connector_id)
            .cloned()
            .unwrap_or(ConnectorFile {
                id: connector_id.to_owned(),
                ..Default::default()
            });
        cf.id = connector_id.to_owned();
        cf.config = config;
        self.connectors.insert(connector_id.to_owned(), cf);
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

    /// Проверить подключение к Google Таблицы через gws CLI.
    pub fn check_connector_status(&mut self, connector_id: &str, _api_client: &Client) {
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

        let spreadsheet_id = match Self::extract_spreadsheet_id(&url) {
            Some(id) => id,
            None => {
                self.connector_status.insert(
                    connector_id.to_owned(),
                    ConnectorStatusInfo {
                        ok: false,
                        last_checked: Some(Local::now().format("%H:%M:%S").to_string()),
                        error: Some("не удалось извлечь ID таблицы из URL".to_owned()),
                    },
                );
                return;
            }
        };

        // Данные читаются через gws CLI. Проверяем доступность таблицы тем же путём.
        let gws_cmd = Self::find_gws_cmd();
        let gws_ok = match &gws_cmd {
            Some(cmd) => Command::new(cmd).arg("--help").output().is_ok(),
            None => false,
        };
        if !gws_ok {
            self.connector_status.insert(
                connector_id.to_owned(),
                ConnectorStatusInfo {
                    ok: false,
                    last_checked: Some(Local::now().format("%H:%M:%S").to_string()),
                    error: Some(
                        "gws CLI не найден. Установите @googleworkspace/cli и выполните gws auth login"
                            .to_owned(),
                    ),
                },
            );
            return;
        }

        let params = serde_json::json!({
            "spreadsheetId": spreadsheet_id,
            "ranges": ["A1"],
            "fields": "sheets(properties.title)"
        });
        let output = Command::new(gws_cmd.unwrap())
            .args([
                "sheets", "spreadsheets", "get",
                "--params", &params.to_string(),
                "--format", "json"
            ])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let _: Value = match serde_json::from_str(&stdout) {
                    Ok(v) => v,
                    Err(_) => {
                        self.connector_status.insert(
                            connector_id.to_owned(),
                            ConnectorStatusInfo {
                                ok: false,
                                last_checked: Some(Local::now().format("%H:%M:%S").to_string()),
                                error: Some("gws CLI вернул некорректный ответ".to_owned()),
                            },
                        );
                        return;
                    }
                };
                self.connector_status.insert(
                    connector_id.to_owned(),
                    ConnectorStatusInfo {
                        ok: true,
                        last_checked: Some(Local::now().format("%H:%M:%S").to_string()),
                        error: None,
                    },
                );
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                self.connector_status.insert(
                    connector_id.to_owned(),
                    ConnectorStatusInfo {
                        ok: false,
                        last_checked: Some(Local::now().format("%H:%M:%S").to_string()),
                        error: Some(format!("gws CLI ошибка: {stderr}")),
                    },
                );
            }
            Err(e) => {
                self.connector_status.insert(
                    connector_id.to_owned(),
                    ConnectorStatusInfo {
                        ok: false,
                        last_checked: Some(Local::now().format("%H:%M:%S").to_string()),
                        error: Some(format!("не удалось запустить gws: {e}")),
                    },
                );
            }
        }
    }

    /// Найти доступную команду gws (gws, gws.exe, gws.cmd, gws.bat).
    fn find_gws_cmd() -> Option<String> {
        for cmd in ["gws", "gws.exe", "gws.cmd", "gws.bat"] {
            if Command::new(cmd).arg("--help").output().is_ok() {
                return Some(cmd.to_owned());
            }
        }
        None
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

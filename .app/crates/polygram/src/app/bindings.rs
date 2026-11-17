use reqwest::blocking::Client;
use serde_json::Value;
use std::collections::HashMap;

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

/// Менеджер API-привязок показателей.
pub struct BindingManager {
    /// Статусы API-привязок (ключ: "{object_id}:{entry_id}").
    pub api_bindings: HashMap<String, BindingStatusInfo>,
    /// Тикер для периодического обновления статусов привязок (каждые ~100 кадров ≈ 2 сек).
    pub api_bindings_tick: u32,
}

impl BindingManager {
    pub fn new() -> Self {
        Self {
            api_bindings: HashMap::new(),
            api_bindings_tick: 0,
        }
    }

    pub fn post_binding(
        &self,
        object_id: String,
        entry_id: String,
        connector: String,
        cell: String,
        interval_secs: u64,
        api_client: &Client,
        api_base: &str,
    ) {
        let api_base = api_base.to_owned();
        let client = api_client.clone();
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

    pub fn delete_binding(
        &self,
        object_id: String,
        entry_id: String,
        api_client: &Client,
        api_base: &str,
    ) {
        let id = format!("{object_id}:{entry_id}");
        let api_base = api_base.to_owned();
        let client = api_client.clone();
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

    pub fn get_binding_statuses(&self, api_client: &Client, api_base: &str) -> Value {
        match api_client.get(format!("{api_base}/api/bindings/status")).send() {
            Ok(resp) => resp.json().unwrap_or_default(),
            Err(_) => serde_json::json!({}),
        }
    }

    /// Получить список всех API-привязок с сервера.
    pub fn get_bindings(&self, api_client: &Client, api_base: &str) -> Vec<Value> {
        match api_client.get(format!("{api_base}/api/bindings")).send() {
            Ok(resp) => {
                let body: Value = resp.json().unwrap_or_default();
                body.get("bindings")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default()
            }
            Err(_) => Vec::new(),
        }
    }

    /// Обновить кэш статусов API-привязок из сервера.
    pub fn refresh_api_bindings(&mut self, api_client: &Client, api_base: &str) {
        let statuses = self.get_binding_statuses(api_client, api_base);
        let bindings = self.get_bindings(api_client, api_base);
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
}

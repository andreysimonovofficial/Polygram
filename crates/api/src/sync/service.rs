//! Сервис синхронизации: состояние, CRUD-операции и жизненный цикл рабочего потока.

use polygram_core::EngineHandle;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;

use super::binding::{ApiBinding, BindingStatus};
use super::connectors::{
    connectors_dir, ensure_default_connectors, load_bindings, ConnectorConfig, ConnectorInfo,
};
use super::source::SourceRegistry;
use super::sync_loop::run_sync_loop;

/// Сервис синхронизации показателей с внешними API.
pub struct SyncService {
    bindings: Arc<Mutex<Vec<ApiBinding>>>,
    connector_configs: Arc<Mutex<Vec<crate::sync::connectors::ConnectorConfig>>>,
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
        let connectors_dir = connectors_dir();
        let _ = fs::create_dir_all(&connectors_dir);

        let mut config_vec: Vec<crate::sync::connectors::ConnectorConfig> = Vec::new();
        let _ = ensure_default_connectors(&connectors_dir);
        if let Ok(entries) = fs::read_dir(&connectors_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(mut cf) =
                            serde_json::from_str::<crate::sync::connectors::ConnectorFile>(&content)
                        {
                            if cf.id.is_empty() {
                                cf.id = path
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or_default()
                                    .to_owned();
                            }
                            config_vec.push(crate::sync::connectors::ConnectorConfig {
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
            bindings: Arc::new(Mutex::new(load_bindings(&connectors_dir))),
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

    pub fn list_bindings(&self) -> Vec<ApiBinding> {
        self.bindings
            .lock()
            .map(|b| b.clone())
            .unwrap_or_default()
    }

    pub fn add_binding(&self, binding: ApiBinding) {
        if let Ok(mut b) = self.bindings.lock() {
            b.retain(|ex| ex.object_id != binding.object_id || ex.entry_id != binding.entry_id);
            b.push(binding.clone());
            let _ = super::connectors::save_bindings(&self.connectors_dir, &b);
        }
        let _ = self.statuses.lock().map(|mut s| {
            s.insert(binding.id.clone(), BindingStatus::default());
        });
    }

    pub fn remove_binding(&self, id: &str) {
        if let Ok(mut b) = self.bindings.lock() {
            b.retain(|ex| ex.id != id);
            let _ = super::connectors::save_bindings(&self.connectors_dir, &b);
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
                        if let Ok(cf) =
                            serde_json::from_str::<crate::sync::connectors::ConnectorFile>(&content)
                        {
                            let id = if cf.id.is_empty() {
                                path.file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or_default()
                                    .to_owned()
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
                    let id = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or_default()
                        .to_owned();
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
            serde_json::from_str::<crate::sync::connectors::ConnectorFile>(&content)
                .unwrap_or(crate::sync::connectors::ConnectorFile {
                    id: config.id.clone(),
                    ..Default::default()
                })
        } else {
            crate::sync::connectors::ConnectorFile {
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
            cfgs.push(crate::sync::connectors::ConnectorConfig {
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
        fs::read_to_string(&path)
            .ok()
            .and_then(|content| {
                serde_json::from_str::<crate::sync::connectors::ConnectorFile>(&content)
                    .ok()
                    .map(|cf| cf.config)
            })
    }

    pub fn update_connector_cache(
        &self,
        id: &str,
        cell: &str,
        value: &polygram_core::types::MetricValue,
    ) {
        let path = self.connectors_dir.join(format!("{}.json", id));
        let mut cf = if let Ok(content) = fs::read_to_string(&path) {
            serde_json::from_str::<crate::sync::connectors::ConnectorFile>(&content)
                .unwrap_or(crate::sync::connectors::ConnectorFile {
                    id: id.to_owned(),
                    ..Default::default()
                })
        } else {
            crate::sync::connectors::ConnectorFile {
                id: id.to_owned(),
                ..Default::default()
            }
        };
        cf.id = id.to_owned();
        let mut cache = cf.cache.as_object().cloned().unwrap_or_default();
        cache.insert(
            cell.to_owned(),
            serde_json::to_value(value).unwrap_or(Value::Null),
        );
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



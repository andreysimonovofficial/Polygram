//! Фоновый цикл опроса привязок и применение результатов.
//!
//! Единственная ответственность — опрашивать привязки по интервалам,
//! считывать значения из источников и применять их к ядру через команды.

use polygram_core::{EngineHandle, types::{Direction, MetricValue}};
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

use super::binding::{ApiBinding, BindingStatus};
use super::connectors::ConnectorConfig;
use super::source::MetricSource;

/// Фоновый цикл опроса привязок.
pub(crate) fn run_sync_loop(
    bindings: Arc<Mutex<Vec<ApiBinding>>>,
    configs: Arc<Mutex<Vec<ConnectorConfig>>>,
    sources: Arc<Mutex<super::source::SourceRegistry>>,
    statuses: Arc<Mutex<HashMap<String, super::binding::BindingStatus>>>,
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
                            super::binding::BindingStatus {
                                ok: false,
                                error: Some(format!(
                                    "запись {} не найдена в объекте {}",
                                    entry_id, object_id
                                )),
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

    let Some(dir) = direction.and_then(|d| d) else {
        let _ = statuses.lock().map(|mut s| {
            s.insert(
                binding_id.clone(),
                BindingStatus {
                    ok: false,
                    error: Some(format!(
                        "запись {} не найдена в объекте {}",
                        entry_id_cmd, object_id_cmd
                    )),
                    ..Default::default()
                },
            );
        });
        return;
    };

    match result {
        Ok(value) => {
            let cmd = match dir {
                Direction::Request => polygram_core::Command::SetRequired {
                    object: object_id_cmd.clone(),
                    entry: entry_id_cmd.clone(),
                    required: value.clone(),
                },
                Direction::Output => polygram_core::Command::SetValue {
                    object: object_id_cmd.clone(),
                    entry: entry_id_cmd.clone(),
                    value: value.clone(),
                },
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
                        if let Ok(mut cf) =
                            serde_json::from_str::<crate::sync::connectors::ConnectorFile>(&content)
                        {
                            let mut cache =
                                cf.cache.as_object().cloned().unwrap_or_default();
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

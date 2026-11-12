use crate::app::PolygramApp;
use polygram_core::{Command, MetricValue, Published};
use polygram_core::{geometry::Geometry, values::DynamicState};
use polygram_core::types::{Direction, MetricKind, NodeKind};
use reqwest::blocking::Client;
use rfd::FileDialog;
use std::sync::Arc;

/// Экспортировать выделенный отдел и его поддерево в .pg файл.
pub fn export_department(app: &mut PolygramApp) {
    if !app.export_department_requested {
        return;
    }
    app.export_department_requested = false;
    let dept_id = app.selected_objects.first().cloned().filter(|id| {
        app.connection.snapshot.as_ref().map_or(false, |s| {
            s.geometry.objects.iter().any(|o| o.id == *id && o.kind == polygram_core::types::NodeKind::Department)
        })
    });
    let Some(dept_id) = dept_id else {
        app.last_status = "Выберите отдел для экспорта".to_owned();
        return;
    };
    let api_base = app.connection.api_base.clone();
    let client = Client::new();
    let result = (|| -> Result<(String, String), String> {
        let resp = client.get(format!("{api_base}/api/document")).send()
            .map_err(|e| e.to_string())?;
        let json = resp.text().map_err(|e| e.to_string())?;
        let doc: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        let objects = doc.get("objects").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let dept_obj = objects.iter().find(|o| o.get("id").and_then(|v| v.as_str()) == Some(&dept_id)).cloned().unwrap_or_default();
        let dept_name = dept_obj.get("name").and_then(|v| v.as_str()).unwrap_or("отдел").to_string();
        let mut subtree_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut stack = vec![dept_id.clone()];
        subtree_ids.insert(dept_id.clone());
        while let Some(cur) = stack.pop() {
            for o in &objects {
                let oid = o.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let parent = o.get("parent").and_then(|v| v.as_str());
                if parent == Some(&cur) && !subtree_ids.contains(&oid) {
                    subtree_ids.insert(oid.clone());
                    stack.push(oid);
                }
            }
        }
        let subtree_objects: Vec<serde_json::Value> = objects.into_iter().filter(|o| {
            o.get("id").and_then(|v| v.as_str()).map_or(false, |id| subtree_ids.contains(id))
        }).collect();
        let mut used_metric_defs: std::collections::HashSet<String> = std::collections::HashSet::new();
        for o in &subtree_objects {
            for key in ["requests", "outputs"] {
                if let Some(arr) = o.get(key).and_then(|v| v.as_array()) {
                    for m in arr {
                        if let Some(md) = m.get("metric_def").and_then(|v| v.as_str()) {
                            used_metric_defs.insert(md.to_string());
                        }
                    }
                }
            }
        }
        let all_metric_defs = doc.get("metric_defs").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let filtered_metric_defs: Vec<serde_json::Value> = all_metric_defs.into_iter().filter(|md| {
            md.get("id").and_then(|v| v.as_str()).map_or(false, |id| used_metric_defs.contains(id))
        }).collect();
        let all_edges = doc.get("edges").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let filtered_edges: Vec<serde_json::Value> = all_edges.into_iter().filter(|e| {
            let from = e.get("from_object").and_then(|v| v.as_str()).unwrap_or("");
            let to = e.get("to_object").and_then(|v| v.as_str()).unwrap_or("");
            subtree_ids.contains(from) && subtree_ids.contains(to)
        }).collect();
        let export_doc = serde_json::json!({
            "format": "polygram/2",
            "name": format!("Отдел: {}", dept_name),
            "metric_defs": filtered_metric_defs,
            "objects": subtree_objects,
            "edges": filtered_edges,
        });
        let json_str = serde_json::to_string_pretty(&export_doc).map_err(|e| e.to_string())?;
        Ok((json_str, dept_name))
    })();
    match result {
        Ok((json, dept_name)) => {
            if let Some(path) = FileDialog::new()
                .set_file_name(format!("{}.pg", dept_name))
                .save_file()
            {
                if let Err(e) = std::fs::write(&path, &json) {
                    app.last_status = format!("Ошибка сохранения отдела: {e}");
                } else {
                    app.last_status = format!("Отдел экспортирован в {}", path.display());
                }
            }
        }
        Err(e) => {
            app.last_status = format!("Ошибка экспорта отдела: {e}");
        }
    }
}

/// Импортировать отдел из .pg файла.
pub fn import_department(app: &mut PolygramApp) {
    if !app.import_department_requested {
        return;
    }
    app.import_department_requested = false;
    if let Some(path) = FileDialog::new().pick_file() {
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(doc) => {
                        let mut cmds: Vec<Command> = Vec::new();
                        let objects = doc.get("objects").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                        let metric_defs = doc.get("metric_defs").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                        for md in &metric_defs {
                            let id = md.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let name = md.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let kind = md.get("kind").and_then(|v| v.as_str()).unwrap_or("number").to_string();
                            let unit = md.get("unit").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let kind_enum = match kind.as_str() {
                                "time" => MetricKind::Time,
                                "date" => MetricKind::Date,
                                "deadline" => MetricKind::Deadline,
                                "percentage" => MetricKind::Percentage,
                                "binary" => MetricKind::Binary,
                                "duration" => MetricKind::Duration,
                                "money" => MetricKind::Money,
                                "text" => MetricKind::Text,
                                "formula" => MetricKind::Formula,
                                _ => MetricKind::Number,
                            };
                            cmds.push(Command::AddMetricDef {
                                id: Some(id),
                                name,
                                kind: kind_enum,
                                unit,
                                api: None,
                            });
                        }
                        for o in &objects {
                            let id = o.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let name = o.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let kind = o.get("kind").and_then(|v| v.as_str()).unwrap_or("process").to_string();
                            let at_x = o.get("at").and_then(|v| v.as_array()).and_then(|a| a.get(0)).and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                            let at_y = o.get("at").and_then(|v| v.as_array()).and_then(|a| a.get(1)).and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                            let parent = o.get("parent").and_then(|v| v.as_str()).map(|s| s.to_string());
                            let kind_enum = match kind.as_str() {
                                "department" => NodeKind::Department,
                                _ => NodeKind::Process,
                            };
                            cmds.push(Command::AddObject {
                                id: Some(id.clone()),
                                name,
                                kind: kind_enum,
                                at: Some([at_x, at_y]),
                                parent,
                                author: None,
                            });
                        }
                        for o in &objects {
                            let oid = o.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            for key in ["requests", "outputs"] {
                                if let Some(arr) = o.get(key).and_then(|v| v.as_array()) {
                                    for m in arr {
                                        let mid = m.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                        let mname = m.get("name").and_then(|v| v.as_str()).unwrap_or(&mid).to_string();
                                        let mdef = m.get("metric_def").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                        let dir = m.get("dir").and_then(|v| v.as_str()).unwrap_or("request").to_string();
                                        let value = MetricValue::Number(m.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0));
                                        let required = MetricValue::Number(m.get("required").and_then(|v| v.as_f64()).unwrap_or(0.0));
                                        let max = m.get("max").and_then(|v| v.as_f64()).map(MetricValue::Number);
                                        let fact = MetricValue::Number(m.get("fact").and_then(|v| v.as_f64()).unwrap_or(0.0));
                                        let dir_enum = if dir == "output" {
                                            Direction::Output
                                        } else {
                                            Direction::Request
                                        };
                                        cmds.push(Command::AddMetricEntry {
                                            object: oid.clone(),
                                            dir: dir_enum,
                                            id: Some(mid),
                                            metric_def: mdef,
                                            name: mname,
                                            value,
                                            required,
                                            max,
                                            fact,
                                        });
                                    }
                                }
                            }
                        }
                        if let Some(edges) = doc.get("edges").and_then(|v| v.as_array()) {
                            for e in edges {
                                let from_object = e.get("from_object").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let from_entry = e.get("from_entry").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let to_object = e.get("to_object").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let to_entry = e.get("to_entry").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let throughput = e.get("throughput").and_then(|v| v.as_f64()).unwrap_or(100.0);
                                cmds.push(Command::Connect {
                                    from_object,
                                    from_entry,
                                    to_object,
                                    to_entry,
                                    throughput,
                                });
                            }
                        }
                        app.apply_batch(cmds);
                        app.last_status = format!("Отдел импортирован из {}", path.display());
                    }
                    Err(e) => {
                        app.last_status = format!("Ошибка импорта отдела: {e}");
                    }
                }
            }
            Err(e) => {
                app.last_status = format!("Ошибка чтения файла: {e}");
            }
        }
    }
}

/// Экспортировать весь граф в .pg файл.
pub fn export_graph(app: &mut PolygramApp) {
    if !app.export_requested {
        return;
    }
    app.export_requested = false;
    if let Some(path) = FileDialog::new().set_file_name("graph.pg").save_file() {
        let api_base = app.connection.api_base.clone();
        let client = Client::new();
        let result = client.get(format!("{api_base}/api/document")).send().and_then(|r| r.text());
        match result {
            Ok(json) => {
                if let Err(e) = std::fs::write(&path, &json) {
                    app.last_status = format!("Ошибка сохранения: {e}");
                } else {
                    app.last_status = format!("Граф сохранён в {}", path.display());
                }
            }
            Err(e) => {
                app.last_status = format!("Ошибка экспорта графа: {e}");
            }
        }
    }
}

/// Импортировать граф из .pg файла как новый отдел (подграф) в текущий граф.
/// ID объектов и показателей переназначаются сервером, чтобы избежать конфликтов.
pub fn import_graph(app: &mut PolygramApp) {
    if !app.import_requested {
        return;
    }
    app.import_requested = false;
    if let Some(path) = FileDialog::new().pick_file() {
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                let dept_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("отдел")
                    .to_owned();
                let api_base = app.connection.api_base.clone();
                let client = Client::new();
                let request = serde_json::json!({
                    "document": serde_json::from_str::<serde_json::Value>(&content).unwrap_or_default(),
                    "name": format!("Отдел: {}", dept_name),
                });
                let result = client
                    .post(format!("{api_base}/api/document/import-department"))
                    .json(&request)
                    .send();
                match result {
                    Ok(resp) => {
                        let status = resp.status();
                        let body: serde_json::Value = resp.json().unwrap_or_default();
                        let ok = body.get("ok").and_then(|v| v.as_bool()).unwrap_or(status.is_success());
                        if ok {
                            let dept = body
                                .get("department")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_owned();
                            app.last_status = if dept.is_empty() {
                                format!("Граф импортирован как отдел из {}", path.display())
                            } else {
                                format!(
                                    "Граф импортирован как отдел «{dept}» из {}",
                                    path.display()
                                )
                            };
                            app.try_connect_once();
                        } else {
                            let error = body
                                .get("error")
                                .and_then(|v| v.as_str())
                                .unwrap_or("неизвестная ошибка");
                            app.last_status =
                                format!("Ошибка импорта графа: {error} (HTTP {status})");
                        }
                    }
                    Err(e) => {
                        app.last_status = format!("Ошибка импорта графа: {e}");
                    }
                }
            }
            Err(e) => {
                app.last_status = format!("Ошибка чтения файла: {e}");
            }
        }
    }
}

/// Защитное значение Published, когда снимок от API ещё не получен.
pub fn empty_published() -> Arc<Published> {
    Arc::new(Published {
        rev: 0,
        dynamic: DynamicState::default(),
        geometry: Geometry::default(),
        metric_defs: vec![],
    })
}

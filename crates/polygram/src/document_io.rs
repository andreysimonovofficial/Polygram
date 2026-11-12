use polygram_core::{Command, MetricValue};
use polygram_core::types::{Direction, MetricKind, NodeKind};
use serde_json::Value;
use std::collections::HashSet;

/// Построить JSON для экспорта отдела и его поддерева из уже полученного документа.
pub fn build_export_department_json(
    doc: &Value,
    dept_id: &str,
) -> Result<(String, String), String> {
    let objects = doc
        .get("objects")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let dept_obj = objects
        .iter()
        .find(|o| o.get("id").and_then(|v| v.as_str()) == Some(dept_id))
        .cloned()
        .unwrap_or_default();
    let dept_name = dept_obj
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("отдел")
        .to_string();

    let mut subtree_ids: HashSet<String> = HashSet::new();
    let mut stack = vec![dept_id.to_string()];
    subtree_ids.insert(dept_id.to_string());
    while let Some(cur) = stack.pop() {
        for o in &objects {
            let oid = o
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let parent = o.get("parent").and_then(|v| v.as_str());
            if parent == Some(&cur) && !subtree_ids.contains(&oid) {
                subtree_ids.insert(oid.clone());
                stack.push(oid);
            }
        }
    }

    let subtree_objects: Vec<Value> = objects
        .into_iter()
        .filter(|o| {
            o.get("id")
                .and_then(|v| v.as_str())
                .map_or(false, |id| subtree_ids.contains(id))
        })
        .collect();

    let mut used_metric_defs: HashSet<String> = HashSet::new();
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

    let all_metric_defs = doc
        .get("metric_defs")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let filtered_metric_defs: Vec<Value> = all_metric_defs
        .into_iter()
        .filter(|md| {
            md.get("id")
                .and_then(|v| v.as_str())
                .map_or(false, |id| used_metric_defs.contains(id))
        })
        .collect();

    let all_edges = doc
        .get("edges")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let filtered_edges: Vec<Value> = all_edges
        .into_iter()
        .filter(|e| {
            let from = e
                .get("from_object")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let to = e
                .get("to_object")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            subtree_ids.contains(from) && subtree_ids.contains(to)
        })
        .collect();

    let export_doc = serde_json::json!({
        "format": "polygram/2",
        "name": format!("Отдел: {}", dept_name),
        "metric_defs": filtered_metric_defs,
        "objects": subtree_objects,
        "edges": filtered_edges,
    });

    let json_str = serde_json::to_string_pretty(&export_doc).map_err(|e| e.to_string())?;
    Ok((json_str, dept_name))
}

/// Построить команды импорта отдела из JSON содержимого файла.
pub fn parse_import_department(content: &str) -> Result<Vec<Command>, String> {
    let doc: Value = serde_json::from_str(content).map_err(|e| e.to_string())?;
    let mut cmds: Vec<Command> = Vec::new();

    let objects = doc
        .get("objects")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let metric_defs = doc
        .get("metric_defs")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

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
        let at_x = o
            .get("at")
            .and_then(|v| v.as_array())
            .and_then(|a| a.get(0))
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let at_y = o
            .get("at")
            .and_then(|v| v.as_array())
            .and_then(|a| a.get(1))
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
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
                    let mname = m
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&mid)
                        .to_string();
                    let mdef = m
                        .get("metric_def")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let dir = m
                        .get("dir")
                        .and_then(|v| v.as_str())
                        .unwrap_or("request")
                        .to_string();
                    let value =
                        MetricValue::Number(m.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0));
                    let required = MetricValue::Number(
                        m.get("required")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0),
                    );
                    let max = m.get("max").and_then(|v| v.as_f64()).map(MetricValue::Number);
                    let fact =
                        MetricValue::Number(m.get("fact").and_then(|v| v.as_f64()).unwrap_or(0.0));
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
            let from_object = e
                .get("from_object")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let from_entry = e
                .get("from_entry")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let to_object = e
                .get("to_object")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let to_entry = e
                .get("to_entry")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
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

    Ok(cmds)
}

/// Построить запрос на импорт графа как отдела.
pub fn build_import_graph_request(content: &str, dept_name: &str) -> Value {
    let document = serde_json::from_str::<Value>(content).unwrap_or_default();
    serde_json::json!({
        "document": document,
        "name": format!("Отдел: {}", dept_name),
    })
}

/// Прочитать файл в строку.
pub fn read_file(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| e.to_string())
}

/// Записать строку в файл.
pub fn write_file(path: &std::path::Path, content: &str) -> Result<(), String> {
    std::fs::write(path, content).map_err(|e| e.to_string())
}

use crate::app::PolygramApp;
use crate::document_io::{
    build_export_department_json, build_import_graph_request, parse_import_department,
    read_file, write_file,
};
use polygram_core::types::NodeKind;
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
            s.geometry.objects.iter().any(|o| o.id == *id && o.kind == NodeKind::Department)
        })
    });
    let Some(dept_id) = dept_id else {
        app.last_status = "Выберите отдел для экспорта".to_owned();
        return;
    };
    let api_base = app.connection.api_base.clone();
    let result = (|| -> Result<(String, String), String> {
        let client = Client::new();
        let resp = client.get(format!("{api_base}/api/document")).send().map_err(|e| e.to_string())?;
        let json = resp.text().map_err(|e| e.to_string())?;
        let doc: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        build_export_department_json(&doc, &dept_id)
    })();
    match result {
        Ok((json, dept_name)) => {
            if let Some(path) = FileDialog::new()
                .set_file_name(format!("{}.pg", dept_name))
                .save_file()
            {
                match write_file(&path, &json) {
                    Ok(()) => {
                        app.last_status = format!("Отдел экспортирован в {}", path.display());
                    }
                    Err(e) => {
                        app.last_status = format!("Ошибка сохранения отдела: {e}");
                    }
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
        match read_file(&path) {
            Ok(content) => {
                match parse_import_department(&content) {
                    Ok(cmds) => {
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
                match write_file(&path, &json) {
                    Ok(()) => {
                        app.last_status = format!("Граф сохранён в {}", path.display());
                    }
                    Err(e) => {
                        app.last_status = format!("Ошибка сохранения: {e}");
                    }
                }
            }
            Err(e) => {
                app.last_status = format!("Ошибка экспорта графа: {e}");
            }
        }
    }
}

/// Импортировать граф из .pg файла как новый отдел (подграф) в текущий граф.
pub fn import_graph(app: &mut PolygramApp) {
    if !app.import_requested {
        return;
    }
    app.import_requested = false;
    if let Some(path) = FileDialog::new().pick_file() {
        match read_file(&path) {
            Ok(content) => {
                let dept_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("отдел")
                    .to_owned();
                let api_base = app.connection.api_base.clone();
                let client = Client::new();
                let request = build_import_graph_request(&content, &dept_name);
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
pub fn empty_published() -> Arc<polygram_core::Published> {
    Arc::new(polygram_core::Published {
        rev: 0,
        dynamic: polygram_core::values::DynamicState::default(),
        geometry: polygram_core::geometry::Geometry::default(),
        metric_defs: vec![],
    })
}

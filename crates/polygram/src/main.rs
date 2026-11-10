//! Окно Polygram: egui-оболочка с меню, палитрой команд, строкой статуса и холстом.
//! Shell — thin-клиент: все команды уходят через HTTP на polygram-api (8080),
//! состояние подтягивается через /api/snapshot. API-сервер работает 24/7,
//! GUI можно пересобирать и перезапускать сколько угодно.

mod app;
mod interaction;
mod menu;
mod palette;
mod panels;
mod status;

use app::PolygramApp;
use eframe::egui::{self, FontData, FontDefinitions};
use polygram_core::{Command, Published};
use rfd::FileDialog;
use polygram_core::{geometry::Geometry, values::DynamicState};
use reqwest::blocking::Client;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

fn load_icon() -> egui::IconData {
    let img = image::load_from_memory(include_bytes!("../assets/favicon.png"))
        .expect("Failed to load favicon.png");
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    egui::IconData {
        rgba: rgba.into_raw(),
        width,
        height,
    }
}

fn register_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    let families = ["inter"];
    let weights = [
        "thin", "extralight", "light", "regular", "medium", "semibold",
        "bold", "extrabold", "black",
    ];

    let fallback_key = load_any_system_font(&mut fonts);
    let unicode_fallback_key = load_unicode_fallback_font(&mut fonts);

    for family in families {
        for weight in weights {
            let key = format!("{family}-{weight}");
            let mut loaded = false;

            if let Some(dir) = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            {
                let path = dir.join(format!("fonts/{key}.ttf"));
                if let Ok(data) = std::fs::read(&path) {
                    fonts.font_data.insert(key.clone(), FontData::from_owned(data));
                    fonts
                        .families
                        .entry(egui::FontFamily::Name(key.clone().into()))
                        .or_default()
                        .push(key.clone());
                    loaded = true;
                }
            }
            if !loaded {
                let path = format!("fonts/{key}.ttf");
                if let Ok(data) = std::fs::read(&path) {
                    fonts.font_data.insert(key.clone(), FontData::from_owned(data));
                    fonts
                        .families
                        .entry(egui::FontFamily::Name(key.clone().into()))
                        .or_default()
                        .push(key.clone());
                    loaded = true;
                }
            }
            if !loaded {
                let path = format!("crates/polygram/fonts/{key}.ttf");
                if let Ok(data) = std::fs::read(&path) {
                    fonts.font_data.insert(key.clone(), FontData::from_owned(data));
                    fonts
                        .families
                        .entry(egui::FontFamily::Name(key.clone().into()))
                        .or_default()
                        .push(key.clone());
                    loaded = true;
                }
            }
            if !loaded {
                if let Some(fb) = &fallback_key {
                    fonts
                        .families
                        .entry(egui::FontFamily::Name(key.clone().into()))
                        .or_default()
                        .push(fb.clone());
                }
            }
            if let Some(uf) = &unicode_fallback_key {
                fonts
                    .families
                    .entry(egui::FontFamily::Name(key.clone().into()))
                    .or_default()
                    .push(uf.clone());
            }
        }
    }

    if let Some(fb) = &fallback_key {
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .push(fb.clone());
    }
    if let Some(uf) = &unicode_fallback_key {
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .push(uf.clone());
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .push(uf.clone());
    }
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .push("inter-regular".to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .push("inter-semibold".to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("inter-regular".to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("inter-semibold".to_owned());

    ctx.set_fonts(fonts);
}

fn load_any_system_font(fonts: &mut FontDefinitions) -> Option<String> {
    for path in [
        "C:/Windows/Fonts/arial.ttf",
        "C:/Windows/Fonts/segoeui.ttf",
        "C:/Windows/Fonts/times.ttf",
    ] {
        if let Ok(data) = std::fs::read(path) {
            let key = "system".to_owned();
            fonts.font_data.insert(key.clone(), FontData::from_owned(data));
            fonts
                .families
                .entry(egui::FontFamily::Name(key.clone().into()))
                .or_default()
                .push(key.clone());
            return Some(key);
        }
    }
    None
}

fn load_unicode_fallback_font(fonts: &mut FontDefinitions) -> Option<String> {
    for path in [
        "C:/Windows/Fonts/seguisym.ttf",
        "C:/Windows/Fonts/segoeui.ttf",
    ] {
        if let Ok(data) = std::fs::read(path) {
            let key = "unicode-fallback".to_owned();
            fonts.font_data.insert(key.clone(), FontData::from_owned(data));
            fonts
                .families
                .entry(egui::FontFamily::Name(key.clone().into()))
                .or_default()
                .push(key.clone());
            return Some(key);
        }
    }
    None
}

impl eframe::App for PolygramApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut close_menu = false;

        // Опрос снимка состояния из API-сервера.
        self.poll_snapshot();

        // Опрос ленты чата.
        self.poll_chat();

        // Периодическая попытка переподключения, если API недоступен (раз в ~50 кадров ≈ 1 сек).
        if !self.connection.api_connected {
            self.connection.reconnect_tick = self.connection.reconnect_tick.saturating_sub(1);
            if self.connection.reconnect_tick == 0 {
                self.connection.reconnect_tick = 150;
                self.try_connect_once();
            }
        }

        // Обновление статусов API-привязок (раз в ~100 кадров ≈ 2 сек).
        self.api_bindings_tick = self.api_bindings_tick.saturating_sub(1);
        if self.api_bindings_tick == 0 {
            self.api_bindings_tick = 100;
            self.refresh_api_bindings();
        }

        let published = self.connection.snapshot.clone().unwrap_or_else(|| {
            Arc::new(Published {
                rev: 0,
                dynamic: DynamicState::default(),
                geometry: Geometry::default(),
                metric_defs: vec![],
            })
        });

        // Считываем события из команды лог (если были).
        // MCP-события больше не приходят в GUI (MCP живёт в polygram-api),
        // команды логируются локально через apply_cmd.

        // Горячая перезагрузка больше не нужна — состояние приходит из API по HTTP.
        menu::top_menu(self, ctx);

        // Диалоги экспорта/импорта отдела (FileDialog — модальное окно).
        if self.export_department_requested {
            self.export_department_requested = false;
            let dept_id = self.selected_objects.first().cloned().filter(|id| {
                self.connection.snapshot.as_ref().map_or(false, |s| {
                    s.geometry.objects.iter().any(|o| o.id == *id && o.kind == polygram_core::types::NodeKind::Department)
                })
            });
            if let Some(dept_id) = dept_id {
                let api_base = self.connection.api_base.clone();
                let client = reqwest::blocking::Client::new();
                let result = (|| -> Result<(String, String), String> {
                    let resp = client.get(format!("{api_base}/api/document")).send()
                        .map_err(|e| e.to_string())?;
                    let json = resp.text().map_err(|e| e.to_string())?;
                    let doc: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
                    // Находим отдел и всех его потомков по parent.
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
                    // Метрики, используемые объектами поддерева.
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
                    // Рёбра внутри поддерева.
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
                        // Открываем диалог сохранения файла.
                        if let Some(path) = FileDialog::new()
                            .set_file_name(format!("{}.pg", dept_name))
                            .save_file()
                        {
                            if let Err(e) = std::fs::write(&path, &json) {
                                self.last_status = format!("Ошибка сохранения отдела: {e}");
                            } else {
                                self.last_status = format!("Отдел экспортирован в {}", path.display());
                            }
                        }
                    }
                    Err(e) => {
                        self.last_status = format!("Ошибка экспорта отдела: {e}");
                    }
                }
            } else {
                self.last_status = "Выберите отдел для экспорта".to_owned();
            }
        }

        if self.import_department_requested {
            self.import_department_requested = false;
            if let Some(path) = FileDialog::new().pick_file() {
                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        // Парсим документ и отправляем команды AddObject/SetObjectParent/AddMetricEntry/Connect.
                        match serde_json::from_str::<serde_json::Value>(&content) {
                            Ok(doc) => {
                                let mut cmds: Vec<Command> = Vec::new();
                                let objects = doc.get("objects").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                                let metric_defs = doc.get("metric_defs").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                                // Сначала создаём metric_defs.
                                for md in &metric_defs {
                                    let id = md.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    let name = md.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    let kind = md.get("kind").and_then(|v| v.as_str()).unwrap_or("number").to_string();
                                    let unit = md.get("unit").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    let kind_enum = match kind.as_str() {
                                        "time" => polygram_core::types::MetricKind::Time,
                                        "percentage" => polygram_core::types::MetricKind::Percentage,
                                        "binary" => polygram_core::types::MetricKind::Binary,
                                        "duration" => polygram_core::types::MetricKind::Duration,
                                        "formula" => polygram_core::types::MetricKind::Formula,
                                        _ => polygram_core::types::MetricKind::Number,
                                    };
                                    cmds.push(Command::AddMetricDef {
                                        id: Some(id),
                                        name,
                                        kind: kind_enum,
                                        unit,
                                        api: None,
                                    });
                                }
                                // Затем объекты (родители पहले, потом дети — Document уже в таком порядке).
                                for o in &objects {
                                    let id = o.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    let name = o.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    let kind = o.get("kind").and_then(|v| v.as_str()).unwrap_or("process").to_string();
                                    let at_x = o.get("at").and_then(|v| v.as_array()).and_then(|a| a.get(0)).and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                                    let at_y = o.get("at").and_then(|v| v.as_array()).and_then(|a| a.get(1)).and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                                    let parent = o.get("parent").and_then(|v| v.as_str()).map(|s| s.to_string());
                                    let kind_enum = match kind.as_str() {
                                        "department" => polygram_core::types::NodeKind::Department,
                                        "portal" => polygram_core::types::NodeKind::Portal,
                                        _ => polygram_core::types::NodeKind::Process,
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
                                // Затем записи показателей.
                                for o in &objects {
                                    let oid = o.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    for key in ["requests", "outputs"] {
                                        if let Some(arr) = o.get(key).and_then(|v| v.as_array()) {
                                            for m in arr {
                                                let mid = m.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                let mname = m.get("name").and_then(|v| v.as_str()).unwrap_or(&mid).to_string();
                                                let mdef = m.get("metric_def").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                let dir = m.get("dir").and_then(|v| v.as_str()).unwrap_or("request").to_string();
                                                let value = m.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                                let required = m.get("required").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                                let max = m.get("max").and_then(|v| v.as_f64());
                                                let fact = m.get("fact").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                                let dir_enum = if dir == "output" {
                                                    polygram_core::types::Direction::Output
                                                } else {
                                                    polygram_core::types::Direction::Request
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
                                // Затем рёбра.
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
                                self.apply_batch(cmds);
                                self.last_status = format!("Отдел импортирован из {}", path.display());
                            }
                            Err(e) => {
                                self.last_status = format!("Ошибка импорта отдела: {e}");
                            }
                        }
                    }
                    Err(e) => {
                        self.last_status = format!("Ошибка чтения файла: {e}");
                    }
                }
            }
        }

        if self.export_requested {
            self.export_requested = false;
            if let Some(path) = FileDialog::new().set_file_name("graph.pg").save_file() {
                let api_base = self.connection.api_base.clone();
                let client = reqwest::blocking::Client::new();
                let result = client.get(format!("{api_base}/api/document")).send().and_then(|r| r.text());
                match result {
                    Ok(json) => {
                        if let Err(e) = std::fs::write(&path, &json) {
                            self.last_status = format!("Ошибка сохранения: {e}");
                        } else {
                            self.last_status = format!("Граф сохранён в {}", path.display());
                        }
                    }
                    Err(e) => {
                        self.last_status = format!("Ошибка экспорта графа: {e}");
                    }
                }
            }
        }

        if self.import_requested {
            self.import_requested = false;
            if let Some(path) = FileDialog::new().pick_file() {
                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        let api_base = self.connection.api_base.clone();
                        let client = reqwest::blocking::Client::new();
                        let result = client.post(format!("{api_base}/api/document/import")).json(&content).send();
                        match result {
                            Ok(_) => {
                                self.last_status = format!("Граф импортирован из {}", path.display());
                                // Запрашиваем новый снимок.
                                self.try_connect_once();
                            }
                            Err(e) => {
                                self.last_status = format!("Ошибка импорта графа: {e}");
                            }
                        }
                    }
                    Err(e) => {
                        self.last_status = format!("Ошибка чтения файла: {e}");
                    }
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            interaction::handle_canvas_interaction(self, ctx, ui, &published, &mut close_menu);
        });

        interaction::context_menu(self, ctx, &published, &mut close_menu);

        interaction::handle_delete_confirm(self, ctx);

        interaction::inline_rename_editor(self, ctx, &published);

        interaction::handle_rename_dialog(self, ctx);

        palette::command_palette(self, ctx);

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            status::status_bar(self, ui);
        });

        if self.show_command_log_panel {
            panels::command_log_panel(self, ctx);
        }

        panels::connection_panel(self, ctx);

        if self.show_connectors_panel {
            panels::connectors_panel(self, ctx);
        }

        if self.show_chat_panel {
            panels::chat_panel(self, ctx);
        }

        if self.connection.warmup_frames > 0 {
            self.connection.warmup_frames -= 1;
            ctx.request_repaint_after(Duration::ZERO);
        }
    }
}

fn main() -> eframe::Result {
    let api_client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    let api_base = "http://127.0.0.1:8080".to_owned();

    // Фоновый опрос снимка состояния из API-сервера.
    let (snapshot_tx, snapshot_rx) = mpsc::channel();
    PolygramApp::start_snapshot_polling(api_client.clone(), api_base.clone(), snapshot_tx);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("Polygram — живая карта системы")
            .with_icon(load_icon()),
        ..Default::default()
    };
    let result = eframe::run_native(
        "Polygram",
        options,
        Box::new(move |cc| {
            register_fonts(&cc.egui_ctx);
            let mut app = PolygramApp::new(snapshot_rx);
            app.try_connect_once();
            Ok(Box::new(app))
        }),
    );

    result
}

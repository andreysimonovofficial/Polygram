//! Окно Polygram: egui-оболочка с меню, палитрой команд, строкой статуса и холстом.
//! Shell владеет `EngineRuntime` и `Canvas`. Команды из UI идут в ядро,
//! снимки (`DynamicState` + `Geometry`) — на холст.
//! Встроены MCP-сервер (stdio) и HTTP-API (8080), делят состояние с GUI через ApiService.

mod app;
mod interaction;
mod menu;
mod palette;
mod panels;
mod server_control;
mod status;

use app::PolygramApp;
use eframe::egui::{self, FontData, FontDefinitions};
use polygram_api::ApiService;
use server_control::ServerControl;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
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

/// Зарегистрировать шрифты Inter (как в Design Tool), чтобы FontFamily::Name("inter-*")
/// не вызывал панику при отрисовке текста. Шрифты из папки `fonts/` рядом с exe
/// (или working dir для cargo run), файлы вида inter-regular.ttf, inter-semibold.ttf и т.д.
/// Также регистрирует Unicode fallback (Segoe UI Symbol), чтобы символы ≡, ⚙ и др.
/// отображались вместо квадратов.
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

            // 1. Папка fonts/ рядом с исполняемым файлом.
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
            // 2. Относительно текущей рабочей директории (для cargo run).
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
            // 3. Относительно crates/polygram/fonts/ (для cargo run из корня репозитория).
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
            // 4. Fallback на любой доступный системный шрифт, чтобы egui не паниковал.
            if !loaded {
                if let Some(fb) = &fallback_key {
                    fonts
                        .families
                        .entry(egui::FontFamily::Name(key.clone().into()))
                        .or_default()
                        .push(fb.clone());
                }
            }
            // 5. Unicode fallback — для символов, которых нет в основном шрифте.
            if let Some(uf) = &unicode_fallback_key {
                fonts
                    .families
                    .entry(egui::FontFamily::Name(key.clone().into()))
                    .or_default()
                    .push(uf.clone());
            }
        }
    }

    // Добавить шрифты в стандартные семейства egui.
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

/// Загрузить любой доступный системный шрифт Windows как гарантированный fallback.
/// Возвращает Some(key) если удалось загрузить, иначе None.
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

/// Загрузить шрифт с широкой поддержкой символов Юникода (Segoe UI Symbol),
/// чтобы символы ≡, ⚙ и др. отображались вместо квадратов.
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
        let published = self.runtime.latest();

        // Считываем события от MCP-сервера и добавляем в лог команд.
        while let Ok(event) = self.mcp_rx.try_recv() {
            self.command_log.push(crate::app::CommandLogEntry {
                cmd: event.cmd,
                result: event.result,
            });
            if self.command_log.len() > 200 {
                self.command_log.remove(0);
            }
        }

        // Горячая перезагрузка состояния из файла автосохранения:
        // HTTP/MCP создаёт объект → polygram-api пишет polygram_auto.pg → GUI подхватывает "на лету".
        if let Ok(metadata) = std::fs::metadata(&self.auto_save_path) {
            if let Ok(mtime) = metadata.modified() {
                if self.last_mtime != Some(mtime) {
                    self.last_mtime = Some(mtime);
                    if let Ok(new_api) = ApiService::load_from_file(&self.auto_save_path) {
                        let mut sc = self.server_control.lock().unwrap();
                        let _ = std::mem::replace(&mut sc.api, Arc::new(new_api));
                        self.runtime = sc.api.handle();
                        self.last_status = "Объект обновлён через HTTP/MCP".to_owned();
                    }
                }
            }
        }

        menu::top_menu(self, ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            interaction::handle_canvas_interaction(self, ctx, ui, &published, &mut close_menu);
        });

        interaction::context_menu(self, ctx, &published, &mut close_menu);

        interaction::inline_rename_editor(self, ctx, &published);

        interaction::handle_rename_dialog(self, ctx);

        palette::command_palette(self, ctx);

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            status::status_bar(self, ui);
        });

        panels::command_log_panel(self, ctx);

        panels::mcp_panel(self, ctx);

        // Прогрев кэша галлей/глифов при старте: форсируем быстрые кадры подряд,
        // чтобы первое панорамирование было гладким без «холодного» старта.
        if self.warmup_frames > 0 {
            self.warmup_frames -= 1;
            ctx.request_repaint_after(Duration::ZERO);
        }
    }
}

/// Путь к файлу автосохранения состояния ядра (рядом с exe).
fn auto_save_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
        .join("polygram_auto.pg")
}

fn main() -> eframe::Result {
    let auto_save_path = auto_save_path();

    // Загрузить существующее состояние, если файл автосохранения есть.
    let api = if auto_save_path.exists() {
        match ApiService::load_from_file(&auto_save_path) {
            Ok(api) => {
                eprintln!("Загружено автосохранение: {}", auto_save_path.display());
                Arc::new(api)
            }
            Err(e) => {
                eprintln!("Не удалось загрузить автосохранение: {e}");
                Arc::new(ApiService::default())
            }
        }
    } else {
        Arc::new(ApiService::default())
    };

    api.set_auto_save_path(auto_save_path.clone());

    let api_for_exit = Arc::clone(&api);
    let auto_save_path_for_exit = auto_save_path.clone();

    let (mcp_tx, mcp_rx) = mpsc::channel();

    let server_control = Arc::new(Mutex::new(ServerControl::new(Arc::clone(&api), mcp_tx)));

    // MCP-сервер (stdio) в отдельном потоке — для Claude Desktop, Open WebUI и др.
    let sc = Arc::clone(&server_control);
    thread::spawn(move || {
        sc.lock().unwrap().start_mcp();
    });

    // HTTP-API запускается отдельным процессом (polygram-api) и пишет состояние
    // в polygram_auto.pg. GUI подхватывает изменения через файл-полинг в update().

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("Polygram — живая карта системы")
            .with_icon(load_icon()),
        ..Default::default()
    };
    let sc_app = Arc::clone(&server_control);
    let result = eframe::run_native(
        "Polygram",
        options,
        Box::new(move |cc| {
            register_fonts(&cc.egui_ctx);
            Ok(Box::new(PolygramApp::new_with_api(
                (*api).clone(),
                mcp_rx,
                sc_app,
            )))
        }),
    );

    // Сохранить состояние при закрытии окна.
    if let Err(e) = api_for_exit.save_to_file(&auto_save_path_for_exit) {
        eprintln!("Не удалось сохранить автосохранение: {e}");
    }

    result
}

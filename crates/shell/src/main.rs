//! Окно Polygram: egui-оболочка с меню, палитрой команд, строкой статуса и холстом.
//! Shell владеет `EngineRuntime` и `Canvas`. Команды из UI идут в ядро,
//! снимки (`DynamicState` + `Geometry`) — на холст.

mod app;
mod interaction;
mod menu;
mod palette;
mod panels;
mod status;

use app::PolygramApp;
use eframe::egui::{self, FontData, FontDefinitions};

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
            let key = format!("{}-{}", family, weight);
            let mut loaded = false;

            // 1. Папка fonts/ рядом с исполняемым файлом.
            if let Some(dir) = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            {
                let path = dir.join(format!("fonts/{}.ttf", key));
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
                let path = format!("fonts/{}.ttf", key);
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
            // 3. Относительно crates/shell/fonts/ (для cargo run из корня репозитория).
            if !loaded {
                let path = format!("crates/shell/fonts/{}.ttf", key);
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
        let published = self.runtime.handle().latest();

        menu::top_menu(self, ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            interaction::handle_canvas_interaction(self, ctx, ui, &published, &mut close_menu);
        });

        interaction::context_menu(self, ctx, &published, &mut close_menu);

        interaction::handle_rename_dialog(self, ctx);

        palette::command_palette(self, ctx);

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            status::status_bar(self, ui);
        });

        panels::command_log_panel(self, ctx);

        ctx.request_repaint();
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("Polygram — живая карта системы")
            .with_icon(load_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "Polygram Shell",
        options,
        Box::new(|cc| {
            register_fonts(&cc.egui_ctx);
            Ok(Box::new(PolygramApp::new()))
        }),
    )
}

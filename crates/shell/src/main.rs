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
fn register_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    let families = ["inter"];
    let weights = [
        "thin", "extralight", "light", "regular", "medium", "semibold",
        "bold", "extrabold", "black",
    ];

    let _fallback_key = load_any_system_font(&mut fonts);
    let _unicode_fallback_key = load_unicode_fallback_font(&mut fonts);

    for family in families {
        for weight in weights {
            let key = format!("{}-{}", family, weight);
            let mut loaded = false;

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
            if !loaded {
                // 3. Относительно crates/shell/fonts/ (для cargo run из корня репозитория).
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
        }
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

fn load_any_system_font(_fonts: &mut FontDefinitions) -> String {
    // egui уже имеет системный шрифт по умолчанию, просто возвращаем ключ fallback.
    "__system_fallback__".to_owned()
}

fn load_unicode_fallback_font(_fonts: &mut FontDefinitions) -> String {
    "__unicode_fallback__".to_owned()
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

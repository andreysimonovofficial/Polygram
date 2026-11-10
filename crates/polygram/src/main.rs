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
use polygram_core::{Published};
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

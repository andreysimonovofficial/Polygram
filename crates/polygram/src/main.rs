//! Окно Polygram: egui-оболочка с меню, палитрой команд, строкой статуса и холстом.
//! Shell — thin-клиент: все команды уходят через HTTP на polygram-api (8080),
//! состояние подтягивается через /api/snapshot. API-сервер работает 24/7,
//! GUI можно пересобирать и перезапускать сколько угодно.

mod app;
mod interaction;
mod menu;
mod palette;
mod panels;
mod document;
mod document_io;
mod command_log_panel;
mod connection_panel;
mod connectors_panel;
mod frontend_log_panel;
mod status;

use app::PolygramApp;
use eframe::egui::{self, FontData, FontDefinitions};
use polygram_core::Published;
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
                    fonts.font_data.insert(key.clone(), std::sync::Arc::new(FontData::from_owned(data)));
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
                    fonts.font_data.insert(key.clone(), std::sync::Arc::new(FontData::from_owned(data)));
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
                    fonts.font_data.insert(key.clone(), std::sync::Arc::new(FontData::from_owned(data)));
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
            fonts.font_data.insert(key.clone(), std::sync::Arc::new(FontData::from_owned(data)));
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
            fonts.font_data.insert(key.clone(), std::sync::Arc::new(FontData::from_owned(data)));
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
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
        self.bindings.api_bindings_tick = self.bindings.api_bindings_tick.saturating_sub(1);
        if self.bindings.api_bindings_tick == 0 {
            self.bindings.api_bindings_tick = 100;
            self.refresh_api_bindings();
        }

        if self.connection.warmup_frames > 0 {
            self.connection.warmup_frames -= 1;
            ctx.request_repaint_after(Duration::ZERO);
        }

        // Поддерживаем перерисовку пока идет автозахват скриншотов.
        if self.screenshot.auto_capture_frames > 0 {
            ctx.request_repaint_after(Duration::ZERO);
        }

        // Применение темы к стилю egui (цвета панелей, окон, виджетов, селекта, отступы, скругление, шрифты).
        let theme = self.canvas.theme;
        ctx.style_mut_of(egui::Theme::Dark, |style| {
            // Отступы и интервалы.
            style.spacing.item_spacing = egui::vec2(theme.item_spacing[0], theme.item_spacing[1]);
            style.spacing.window_margin = egui::Margin {
                left: theme.window_margin[0] as i8,
                right: theme.window_margin[0] as i8,
                top: theme.window_margin[1] as i8,
                bottom: theme.window_margin[1] as i8,
            };
            style.spacing.menu_margin = egui::Margin {
                left: theme.menu_margin[0] as i8,
                right: theme.menu_margin[0] as i8,
                top: theme.menu_margin[1] as i8,
                bottom: theme.menu_margin[1] as i8,
            };
            style.spacing.indent = theme.indent;
            style.spacing.combo_height = theme.combo_height;
            style.spacing.slider_width = theme.slider_width;
            style.visuals.resize_corner_size = theme.resize_corner_size;
            // Цвета панелей и окон.
            style.visuals.window_fill = theme.panel_bg;
            style.visuals.panel_fill = theme.panel_bg;
            style.visuals.faint_bg_color = theme.faint_bg_color;
            style.visuals.extreme_bg_color = theme.extreme_bg_color;
            style.visuals.code_bg_color = theme.code_bg_color;

            // Обводки окон/панелей.
            style.visuals.window_stroke = egui::Stroke::new(1.0, theme.panel_border);
            style.visuals.window_corner_radius = theme.window_rounding();

            // Виджеты.
            style.visuals.widgets.noninteractive.bg_fill = theme.panel_bg;
            style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, theme.panel_border);
            style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, theme.panel_text);
            style.visuals.widgets.noninteractive.corner_radius = theme.widget_rounding_egui();
            style.visuals.widgets.inactive.bg_fill = theme.panel_bg;
            style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, theme.panel_border);
            style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, theme.panel_text);
            style.visuals.widgets.inactive.corner_radius = theme.widget_rounding_egui();
            style.visuals.widgets.hovered.bg_fill = theme.menu_hover;
            style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, theme.accent_secondary);
            style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, theme.panel_text);
            style.visuals.widgets.hovered.corner_radius = theme.widget_rounding_egui();
            style.visuals.widgets.active.bg_fill = theme.accent;
            style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, theme.accent);
            style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, theme.panel_bg);
            style.visuals.widgets.active.corner_radius = theme.widget_rounding_egui();
            style.visuals.widgets.open.corner_radius = theme.widget_rounding_egui();

            // Выделение, ссылки, предупреждения, ошибки.
            style.visuals.selection.bg_fill = theme.accent;
            style.visuals.selection.stroke = egui::Stroke::new(1.0, theme.accent);
            style.visuals.hyperlink_color = theme.link_color;
            style.visuals.warn_fg_color = theme.status_warning;
            style.visuals.error_fg_color = theme.status_deficit;

            // Тени окон и popup.
            style.visuals.window_shadow = egui::epaint::Shadow {
                offset: [theme.shadow_offset[0] as i8, theme.shadow_offset[1] as i8],
                blur: theme.shadow_blur as u8,
                spread: 0,
                color: theme.shadow_color,
            };
            style.visuals.popup_shadow = egui::epaint::Shadow {
                offset: [theme.shadow_offset[0] as i8, theme.shadow_offset[1] as i8],
                blur: theme.shadow_blur as u8,
                spread: 0,
                color: theme.shadow_color,
            };

            // Текстовые стили / шрифты.
            style.text_styles.get_mut(&egui::TextStyle::Body).map(|f| f.size = theme.text_size_body);
            style.text_styles.get_mut(&egui::TextStyle::Button).map(|f| f.size = theme.text_size_button);
            style.text_styles.get_mut(&egui::TextStyle::Small).map(|f| f.size = theme.text_size_small);
            style.text_styles.get_mut(&egui::TextStyle::Heading).map(|f| f.size = theme.text_size_heading);
            style.text_styles.get_mut(&egui::TextStyle::Monospace).map(|f| f.size = theme.text_size_monospace);
        });
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let mut close_menu = false;

        let published = self.connection.snapshot.clone().unwrap_or_else(|| {
            Arc::new(Published {
                rev: 0,
                dynamic: DynamicState::default(),
                geometry: Geometry::default(),
                metric_defs: vec![],
            })
        });

        let theme = self.canvas.theme;

        // Горячая перезагрузка больше не нужна — состояние приходит из API по HTTP.
        menu::top_menu(self, ui, &theme);

        document::export_department(self);

        document::import_department(self);

        document::export_graph(self);

        document::import_graph(self);

        document::open_project(self);

        document::save_project(self);

        document::save_project_as(self);

        egui::Panel::bottom("status").show(ui, |ui| {
            status::status_bar(self, ui);
        });

        panels::connection_panel(self, ui, &theme);

        if self.show_connectors_panel {
            panels::connectors_panel(self, ui, &theme);
        }

        if self.show_chat_panel {
            panels::chat_panel(self, ui, &theme);
        }

        if self.show_command_log_panel {
            panels::command_log_panel(self, ui, &theme);
        }

        if self.show_frontend_log_panel {
            panels::frontend_log_panel(self, ui, &theme);
        }

        egui::CentralPanel::default().show(ui, |ui| {
            if self.show_metrics_screen {
                interaction::container_menu::render_metrics_screen(
                    self, &ctx, ui, &published, &theme,
                );
            } else {
                interaction::handle_canvas_interaction(
                    self, &ctx, ui, &published, &mut close_menu, &theme,
                );
            }
        });

        // Подтверждение закрытия с несохранёнными изменениями.
        if self.close_confirmation_pending {
            self.close_confirmation_pending = false;
            egui::Window::new("Несохранённые изменения")
                .collapsible(false)
                .resizable(false)
                .show(&ctx, |ui| {
                    ui.label("Есть несохранённые изменения. Сохранить проект перед выходом?");
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Сохранить и выйти").clicked() {
                            if self.last_opened_file.is_some() {
                                self.save_project_requested = true;
                            } else {
                                self.save_project_as_requested = true;
                            }
                            // Даем время на сохранение перед выходом.
                            ctx.request_repaint();
                        }
                        if ui.button("Выйти без сохранения").clicked() {
                            std::process::exit(0);
                        }
                        if ui.button("Отмена").clicked() {
                            // Не закрываем.
                        }
                    });
                });
        }

        interaction::context_menu(self, &ctx, &published, &mut close_menu);

        // Снимок экрана (блокирующая операция GDI, вызывается редко).
        if self.screenshot.requested || self.screenshot.auto_capture_frames > 0 {
            self.screenshot.requested = false;
            self.screenshot.auto_capture_frames = self.screenshot.auto_capture_frames.saturating_sub(1);
            match self.screenshot.capture() {
                Ok(path) => {
                    self.screenshot.result = Some(Ok(path.clone()));
                    self.last_status = format!("Снимок экрана сохранён: {}", path.display());
                    self.add_event_log("info", "screenshot", &format!("Сохранён: {}", path.display()), None);
                }
                Err(e) => {
                    self.screenshot.result = Some(Err(e.clone()));
                    self.last_status = format!("Ошибка скриншота: {}", e);
                    self.add_event_log("error", "screenshot", &e, None);
                }
            }
        }

        // Окно детальных настроек темы.
        if self.show_settings {
            if crate::app::settings::render_settings(self, ui) {
                self.ui_config_save();
            }
        }

        interaction::handle_delete_confirm(self, &ctx);

        interaction::handle_reparent_confirm(self, &ctx);

        interaction::inline_rename_editor(self, &ctx, &published);

        interaction::handle_rename_dialog(self, &ctx);

        palette::command_palette(self, &ctx);
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
            app.start_api_process();
            app.try_connect_once();

            // Если есть последний открытый файл — открываем его при старте.
            if let Some(ref path) = app.last_opened_file {
                if std::path::Path::new(path).exists() {
                    app.open_project_requested = true;
                }
            }

            Ok(Box::new(app))
        }),
    );

    result
}

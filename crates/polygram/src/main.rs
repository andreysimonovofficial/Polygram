//! РћРєРЅРѕ Polygram: egui-РѕР±РѕР»РѕС‡РєР° СЃ РјРµРЅСЋ, РїР°Р»РёС‚СЂРѕР№ РєРѕРјР°РЅРґ, СЃС‚СЂРѕРєРѕР№ СЃС‚Р°С‚СѓСЃР° Рё С…РѕР»СЃС‚РѕРј.
//! Shell вЂ” thin-РєР»РёРµРЅС‚: РІСЃРµ РєРѕРјР°РЅРґС‹ СѓС…РѕРґСЏС‚ С‡РµСЂРµР· HTTP РЅР° polygram-api (8080),
//! СЃРѕСЃС‚РѕСЏРЅРёРµ РїРѕРґС‚СЏРіРёРІР°РµС‚СЃСЏ С‡РµСЂРµР· /api/snapshot. API-СЃРµСЂРІРµСЂ СЂР°Р±РѕС‚Р°РµС‚ 24/7,
//! GUI РјРѕР¶РЅРѕ РїРµСЂРµСЃРѕР±РёСЂР°С‚СЊ Рё РїРµСЂРµР·Р°РїСѓСЃРєР°С‚СЊ СЃРєРѕР»СЊРєРѕ СѓРіРѕРґРЅРѕ.

mod app;
mod command_log_panel;
mod connection_panel;
mod connectors_panel;
mod document;
mod document_io;
mod frontend_log_panel;
mod interaction;
mod menu;
mod palette;
mod panels;
mod status;

use app::PolygramApp;
use eframe::egui::{self, ColorImage};
use polygram_core::Published;
use polygram_core::{geometry::Geometry, values::DynamicState};
use reqwest::blocking::Client;
use std::path::PathBuf;
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

impl eframe::App for PolygramApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // РћРїСЂРѕСЃ СЃРЅРёРјРєР° СЃРѕСЃС‚РѕСЏРЅРёСЏ РёР· API-СЃРµСЂРІРµСЂР°.
        self.poll_snapshot();

        // РҐР°Р±Р»РѕСЏ С‡Р°С‚Р°: виртуальная мышь и клавиатура (управляются через MCP/API).
        self.inject_virtual_input(ctx);

        // РћРїСЂРѕСЃ Р»РµРЅС‚С‹ С‡Р°С‚Р°.
        self.poll_chat();

        // РџРµСЂРёРѕРґРёС‡РµСЃРєР°СЏ РїРѕРїС‹С‚РєР° РїРµСЂРµРїРѕРґРєР»СЋС‡РµРЅРёСЏ, РµСЃР»Рё API РЅРµРґРѕСЃС‚СѓРїРµРЅ (СЂР°Р· РІ ~50 РєР°РґСЂРѕРІ в‰€ 1 СЃРµРє).
        if !self.connection.api_connected {
            self.connection.reconnect_tick = self.connection.reconnect_tick.saturating_sub(1);
            if self.connection.reconnect_tick == 0 {
                self.connection.reconnect_tick = 150;
                self.try_connect_once();
            }
        }

        // РћР±РЅРѕРІР»РµРЅРёРµ СЃС‚Р°С‚СѓСЃРѕРІ API-РїСЂРёРІСЏР·РѕРє (СЂР°Р· РІ ~100 РєР°РґСЂРѕРІ в‰€ 2 СЃРµРє).
        self.bindings.api_bindings_tick = self.bindings.api_bindings_tick.saturating_sub(1);
        if self.bindings.api_bindings_tick == 0 {
            self.bindings.api_bindings_tick = 100;
            self.refresh_api_bindings();
        }

        if self.connection.warmup_frames > 0 {
            self.connection.warmup_frames -= 1;
            ctx.request_repaint_after(Duration::ZERO);
        }

        // РџСЂРёРјРµРЅРµРЅРёРµ С‚РµРјС‹ Рє СЃС‚РёР»СЋ egui (С†РІРµС‚Р° РїР°РЅРµР»РµР№, РѕРєРѕРЅ, РІРёРґР¶РµС‚РѕРІ, СЃРµР»РµРєС‚Р°, РѕС‚СЃС‚СѓРїС‹, СЃРєСЂСѓРіР»РµРЅРёРµ, С€СЂРёС„С‚С‹).
        let theme = self.canvas.theme;
        ctx.style_mut_of(egui::Theme::Dark, |style| {
            // РћС‚СЃС‚СѓРїС‹ Рё РёРЅС‚РµСЂРІР°Р»С‹.
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
            // Р¦РІРµС‚Р° РїР°РЅРµР»РµР№ Рё РѕРєРѕРЅ.
            style.visuals.window_fill = theme.panel_bg;
            style.visuals.panel_fill = theme.panel_bg;
            style.visuals.faint_bg_color = theme.faint_bg_color;
            style.visuals.extreme_bg_color = theme.extreme_bg_color;
            style.visuals.code_bg_color = theme.code_bg_color;

            // РћР±РІРѕРґРєРё РѕРєРѕРЅ/РїР°РЅРµР»РµР№.
            style.visuals.window_stroke = egui::Stroke::new(1.0, theme.panel_border);
            style.visuals.window_corner_radius = theme.window_rounding();

            // Р’РёРґР¶РµС‚С‹.
            style.visuals.widgets.noninteractive.bg_fill = theme.panel_bg;
            style.visuals.widgets.noninteractive.bg_stroke =
                egui::Stroke::new(1.0, theme.panel_border);
            style.visuals.widgets.noninteractive.fg_stroke =
                egui::Stroke::new(1.0, theme.panel_text);
            style.visuals.widgets.noninteractive.corner_radius = theme.widget_rounding_egui();
            style.visuals.widgets.inactive.bg_fill = theme.panel_bg;
            style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, theme.panel_border);
            style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, theme.panel_text);
            style.visuals.widgets.inactive.corner_radius = theme.widget_rounding_egui();
            style.visuals.widgets.hovered.bg_fill = theme.menu_hover;
            style.visuals.widgets.hovered.bg_stroke =
                egui::Stroke::new(1.0, theme.accent_secondary);
            style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, theme.panel_text);
            style.visuals.widgets.hovered.corner_radius = theme.widget_rounding_egui();
            style.visuals.widgets.active.bg_fill = theme.accent;
            style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, theme.accent);
            style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, theme.panel_bg);
            style.visuals.widgets.active.corner_radius = theme.widget_rounding_egui();
            style.visuals.widgets.open.corner_radius = theme.widget_rounding_egui();

            // Р’С‹РґРµР»РµРЅРёРµ, СЃСЃС‹Р»РєРё, РїСЂРµРґСѓРїСЂРµР¶РґРµРЅРёСЏ, РѕС€РёР±РєРё.
            style.visuals.selection.bg_fill = theme.accent;
            style.visuals.selection.stroke = egui::Stroke::new(1.0, theme.accent);
            style.visuals.hyperlink_color = theme.link_color;
            style.visuals.warn_fg_color = theme.status_warning;
            style.visuals.error_fg_color = theme.status_deficit;

            // РўРµРЅРё РѕРєРѕРЅ Рё popup.
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

            // РўРµРєСЃС‚РѕРІС‹Рµ СЃС‚РёР»Рё / С€СЂРёС„С‚С‹.
            style
                .text_styles
                .get_mut(&egui::TextStyle::Body)
                .map(|f| f.size = theme.text_size_body);
            style
                .text_styles
                .get_mut(&egui::TextStyle::Button)
                .map(|f| f.size = theme.text_size_button);
            style
                .text_styles
                .get_mut(&egui::TextStyle::Small)
                .map(|f| f.size = theme.text_size_small);
            style
                .text_styles
                .get_mut(&egui::TextStyle::Heading)
                .map(|f| f.size = theme.text_size_heading);
            style
                .text_styles
                .get_mut(&egui::TextStyle::Monospace)
                .map(|f| f.size = theme.text_size_monospace);
        });
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Автоматический снимок экрана при изменении ревизии состояния (только если режим включён).
        let current_rev = self
            .connection
            .snapshot
            .as_ref()
            .map(|s| s.rev)
            .unwrap_or(0);
        let auto_screenshot = self
            .connection
            .snapshot
            .as_ref()
            .map(|s| s.auto_screenshot)
            .unwrap_or(false);
        if auto_screenshot && current_rev > 0 && current_rev > self.last_screenshot_rev {
            self.last_screenshot_rev = current_rev;
            self.screenshot_requested = true;
            ctx.request_repaint();
        }

        let mut close_menu = false;

        let published = self.connection.snapshot.clone().unwrap_or_else(|| {
            Arc::new(Published {
                rev: 0,
                dynamic: DynamicState::default(),
                geometry: Geometry::default(),
                metric_defs: vec![],
                camera_fit_requested: false,
                auto_screenshot: false,
                virtual_input: polygram_core::VirtualInputState::default(),
            })
        });

        // Применить "Вписать" камеру, если ядро запросило вписывание объектов в область просмотра.
        if published.camera_fit_requested {
            let viewport = ui.available_rect_before_wrap();
            crate::interaction::camera::fit_camera_to_objects(
                &mut self.canvas.camera,
                &published.geometry,
                viewport,
                self.canvas.cell_size,
            );
            ctx.request_repaint();
        }

        let theme = self.canvas.theme;

        // Р“РѕСЂСЏС‡Р°СЏ РїРµСЂРµР·Р°РіСЂСѓР·РєР° Р±РѕР»СЊС€Рµ РЅРµ РЅСѓР¶РЅР° вЂ” СЃРѕСЃС‚РѕСЏРЅРёРµ РїСЂРёС…РѕРґРёС‚ РёР· API РїРѕ HTTP.
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
                    self,
                    &ctx,
                    ui,
                    &published,
                    &mut close_menu,
                    &theme,
                );
            }

            // Виртуальный курсор мыши — рисуется поверх холста внутри CentralPanel,
            // чтобы агент видел, куда «кликает» или «тащит».
            let vi = &self.virtual_input;
            // Координаты API — пиксели экрана, egui — логические пиксели. Делим на scale.
            let scale = ctx.pixels_per_point();
            let pos = self.virtual_cursor_pos / scale;
            if vi.target_x != 0.0
                || vi.target_y != 0.0
                || vi.mouse_left_pressed
                || vi.mouse_right_pressed
                || !vi.path_points.is_empty()
            {
                let cursor_color = if vi.mouse_left_pressed || vi.mouse_right_pressed {
                    egui::Color32::from_rgb(255, 60, 60)
                } else {
                    egui::Color32::from_rgb(0, 255, 140)
                };
                // Пульсация для лучшей видимости.
                let pulse = (ctx.input(|i| i.time) as f32 * 8.0).sin().abs() * 3.0 + 1.0;
                let r = 14.0 + pulse;
                // Рисуем на переднем слое в логических координатах egui.
                let screen_painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("virtual_shapes")));
                // Внешнее белое кольцо.
                screen_painter.circle_stroke(pos, r + 4.0, egui::Stroke::new(3.0, egui::Color32::WHITE));
                // Заполненный круг.
                screen_painter.circle_filled(pos, r, cursor_color);
                // Перекрестие.
                screen_painter.line_segment(
                    [pos + egui::Vec2::new(-20.0, 0.0), pos + egui::Vec2::new(20.0, 0.0)],
                    egui::Stroke::new(3.0, cursor_color),
                );
                screen_painter.line_segment(
                    [pos + egui::Vec2::new(0.0, -20.0), pos + egui::Vec2::new(0.0, 20.0)],
                    egui::Stroke::new(3.0, cursor_color),
                );
                // Подпись координат.
                screen_painter.text(
                    pos + egui::Vec2::new(25.0, -20.0),
                    egui::Align2::LEFT_TOP,
                    format!("({}, {})", pos.x as i32, pos.y as i32),
                    egui::FontId::new(18.0, egui::FontFamily::Proportional),
                    cursor_color,
                );
            }

            // Рисование фигуры курсором (от точки нажатия до цели draw_shape).
            if let Some((kind, x2, y2)) = &vi.draw_shape {
                if let Some(start) = self.virtual_draw_start {
                    let end = egui::Pos2::new(*x2 as f32, *y2 as f32);
                    let shape_color = egui::Color32::from_rgb(0, 200, 255);
                    let stroke = egui::Stroke::new(3.0, shape_color);
                    let rect = egui::Rect::from_two_pos(start, end);
                let screen_painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("virtual_cursor")));
                    match *kind {
                        0 => {
                            // Эллипс — приближаем многоугольником из 32 отрезков.
                            let center = rect.center();
                            let radii = rect.size() * 0.5;
                            let n = 32;
                            for i in 0..n {
                                let a1 = (i as f32) / (n as f32) * 6.2831853;
                                let a2 = ((i + 1) as f32) / (n as f32) * 6.2831853;
                                let p1 = center + egui::Vec2::new(a1.cos() * radii.x, a1.sin() * radii.y);
                                let p2 = center + egui::Vec2::new(a2.cos() * radii.x, a2.sin() * radii.y);
                                screen_painter.line_segment([p1, p2], stroke);
                            }
                        }
                        1 => {
                            // Прямоугольник — 4 отрезка.
                            let tl = rect.left_top();
                            let tr = rect.right_top();
                            let bl = rect.left_bottom();
                            let br = rect.right_bottom();
                            screen_painter.line_segment([tl, tr], stroke);
                            screen_painter.line_segment([tr, br], stroke);
                            screen_painter.line_segment([br, bl], stroke);
                            screen_painter.line_segment([bl, tl], stroke);
                        }
                        _ => {}
                    }
                    // Подпись типа фигуры.
                    screen_painter.text(
                        end + egui::Vec2::new(10.0, -10.0),
                        egui::Align2::LEFT_TOP,
                        if *kind == 0 { "эллипс" } else { "прямоугольник" },
                        egui::FontId::new(14.0, egui::FontFamily::Proportional),
                        shape_color,
                    );
                }
            }
        });

        // РџРѕРґС‚РІРµСЂР¶РґРµРЅРёРµ Р·Р°РєСЂС‹С‚РёСЏ СЃ РЅРµСЃРѕС…СЂР°РЅС‘РЅРЅС‹РјРё РёР·РјРµРЅРµРЅРёСЏРјРё.
        if self.close_confirmation_pending {
            self.close_confirmation_pending = false;
            egui::Window::new("РќРµСЃРѕС…СЂР°РЅС‘РЅРЅС‹Рµ РёР·РјРµРЅРµРЅРёСЏ")
                .collapsible(false)
                .resizable(false)
                .show(&ctx, |ui| {
                    ui.label("Р•СЃС‚СЊ РЅРµСЃРѕС…СЂР°РЅС‘РЅРЅС‹Рµ РёР·РјРµРЅРµРЅРёСЏ. РЎРѕС…СЂР°РЅРёС‚СЊ РїСЂРѕРµРєС‚ РїРµСЂРµРґ РІС‹С…РѕРґРѕРј?");
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("РЎРѕС…СЂР°РЅРёС‚СЊ Рё РІС‹Р№С‚Рё").clicked() {
                            if self.last_opened_file.is_some() {
                                self.save_project_requested = true;
                            } else {
                                self.save_project_as_requested = true;
                            }
                            // Р”Р°РµРј РІСЂРµРјСЏ РЅР° СЃРѕС…СЂР°РЅРµРЅРёРµ РїРµСЂРµРґ РІС‹С…РѕРґРѕРј.
                            ctx.request_repaint();
                        }
                        if ui.button("Р’С‹Р№С‚Рё Р±РµР· СЃРѕС…СЂР°РЅРµРЅРёСЏ").clicked() {
                            std::process::exit(0);
                        }
                        if ui.button("РћС‚РјРµРЅР°").clicked() {
                            // РќРµ Р·Р°РєСЂС‹РІР°РµРј.
                        }
                    });
                });
        }

        interaction::context_menu(self, &ctx, &published, &mut close_menu);

        // Снимок экрана через egui API (только содержимое окна приложения).
        if self.screenshot_requested {
            let result = self.screenshot_result.clone();
            self.screenshot_requested = false;
            let ctx_repaint = ctx.clone();
            ctx.request_screenshot(move |image: Arc<ColorImage>| {
                let dir = PathBuf::from("screenshots");
                let res = crate::app::snapshot::save_screenshot(&image, &dir);
                let mut guard = result.lock().unwrap();
                *guard = Some(res.clone().map(|p| Ok(p)).unwrap_or_else(|e| Err(e)));
                drop(guard);
                ctx_repaint.request_repaint();
            });
        }

        // Показать статус последнего скриншота.
        let screenshot_res = {
            let mut guard = self.screenshot_result.lock().unwrap();
            guard.take()
        };
        if let Some(res) = screenshot_res {
            match res {
                Ok(path) => {
                    self.last_status = format!("Снимок экрана сохранён: {}", path.display());
                    self.add_event_log(
                        "info",
                        "screenshot",
                        &format!("Сохранён: {}", path.display()),
                        None,
                    );
                }
                Err(e) => {
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

        interaction::handle_reparent_confirm(self, &ctx, &published);

        interaction::handle_create_subdepartment(self, &ctx);

        interaction::inline_rename_editor(self, &ctx, &published);

        interaction::handle_rename_dialog(self, &ctx);

        palette::command_palette(self, &ctx);
    }
}
/// РЎРѕС…СЂР°РЅРёС‚СЊ РєР°РґСЂ egui ColorImage РєР°Рє PNG РІ СѓРєР°Р·Р°РЅРЅСѓСЋ РґРёСЂРµРєС‚РѕСЂРёСЋ.

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
            crate::app::fonts::register_fonts(&cc.egui_ctx);
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

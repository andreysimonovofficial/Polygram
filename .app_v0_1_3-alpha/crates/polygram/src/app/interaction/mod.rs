//! Взаимодействие с пользователем: контекстное меню, диалоги показателей.

pub mod add_metric;
pub mod container_menu;
pub mod reparent_confirm;
pub mod rename;

use polygram_core::id_allocator::allocate_id;

/// Контекстное меню по правой кнопке мыши на холсте.
pub fn context_menu(
    app: &mut super::PolygramApp,
    ctx: &egui::Context,
    published: &polygram_core::runtime::Published,
    theme: polygram_canvas::Theme,
) {
    let Some(pos) = app.right_click else {
        return;
    };

    let mut items: Vec<(&str, egui_lucide::Lucide)> = vec![
        ("Создать Процесс", egui_lucide::Lucide::SquareEqual),
        ("Создать Отдел", egui_lucide::Lucide::Group),
    ];
    let mut add_metric_request = false;
    let mut add_metric_result = false;
    let mut open_metrics = false;

    if let Some(ref object_id) = app.right_click_object {
        if let Some(obj) = app.canvas.objects.iter().find(|o| o.id == *object_id) {
            if obj.kind == polygram_core::types::NodeKind::Process {
                items.push(("Добавить в Запрос", egui_lucide::Lucide::Plus));
                items.push(("Добавить в Результат", egui_lucide::Lucide::Plus));
                items.push(("Показатели", egui_lucide::Lucide::Settings));
                items.push(("Переименовать", egui_lucide::Lucide::Pencil));
            } else {
                items.push(("Переименовать", egui_lucide::Lucide::Pencil));
            }
        }
    }
    let mut close = false;
    let mut create_department = false;
    let mut create_process = false;
    let mut rename = false;

    let mut item_rects: Vec<egui::Rect> = Vec::with_capacity(items.len());
    let mut need_clamp = false;
    let mut clamped_pos = pos;

    egui::Window::new("")
        .title_bar(false)
        .fixed_pos(pos)
        .movable(false)
        .resizable(false)
        .collapsible(false)
        .frame(
            egui::Frame::new()
                .fill(theme.menu_frame_fill)
                .stroke(egui::Stroke::new(
                    theme.menu_frame_stroke_width,
                    theme.menu_frame_stroke_color,
                ))
                .corner_radius(theme.menu_frame_rounding)
                .inner_margin(theme.menu_frame_margin)
                .shadow(theme.shadow),
        )
        .show(ctx, |ui| {
            let style = ui.style();
            let font_id = egui::TextStyle::Button.resolve(style);
            let button_padding = style.spacing.button_padding;
            let item_rounding = theme.menu_item_rounding;
            let item_bg_fill = theme.menu_item_bg_fill;
            let hovered_bg_fill = egui::Color32::from_rgb(52, 52, 52);
            let hover_pos = ctx.input(|i| i.pointer.hover_pos());

            let icon_size = 16.0;
            let icon_gap = 8.0;

            let max_text_width = ui.fonts_mut(|f| {
                items
                    .iter()
                    .map(|(item, _)| {
                        let galley = f.layout_no_wrap(item.to_string(), font_id.clone(), egui::Color32::WHITE);
                        galley.size().x
                    })
                    .fold(0.0, f32::max)
            });
            let max_width = icon_size + icon_gap + max_text_width + 2.0 * button_padding.x;
            let item_height = (font_id.size + 2.0 * button_padding.y) * 1.5;
            let menu_height = item_height * items.len() as f32;

            let screen_rect = ctx.viewport_rect();
            let mut cx = pos.x;
            if cx + max_width > screen_rect.right() {
                cx = screen_rect.right() - max_width;
            }
            if cx < screen_rect.left() {
                cx = screen_rect.left();
            }
            let mut cy = pos.y;
            if cy + menu_height > screen_rect.bottom() {
                cy = screen_rect.bottom() - menu_height;
            }
            if cy < screen_rect.top() {
                cy = screen_rect.top();
            }
            if cx != pos.x || cy != pos.y {
                need_clamp = true;
                clamped_pos = egui::pos2(cx, cy);
            }

            for (item, icon) in &items {
                let cursor = ui.cursor();
                let item_rect = egui::Rect::from_min_size(
                    cursor.min,
                    egui::vec2(max_width, item_height),
                );
                item_rects.push(item_rect);

                let is_hovered = hover_pos.map_or(false, |p| item_rect.contains(p));
                let bg = if is_hovered {
                    hovered_bg_fill
                } else {
                    item_bg_fill
                };
                ui.painter().rect_filled(item_rect, item_rounding, bg);

                let response = ui.interact(item_rect, ui.scope_id().with(item), egui::Sense::click());

                let icon_y_rel = (item_height - icon_size) / 2.0;
                let icon_rect = egui::Rect::from_min_size(
                    item_rect.min + egui::vec2(button_padding.x, icon_y_rel),
                    egui::vec2(icon_size, icon_size),
                );
                icon.color(egui::Color32::WHITE).size(icon_size).image().paint_at(ui, icon_rect);

                let text_color = egui::Color32::WHITE;
                let galley = ui.fonts_mut(|f| {
                    f.layout_no_wrap(item.to_string(), font_id.clone(), text_color)
                });
                let text_y_rel = (item_height - galley.size().y) / 2.0;
                let text_pos = item_rect.min + egui::vec2(
                    button_padding.x + icon_size + icon_gap,
                    text_y_rel,
                );
                ui.painter().galley(text_pos, galley, text_color);

                if response.clicked() {
                    close = true;
                    if *item == "Создать Отдел" {
                        create_department = true;
                    } else if *item == "Создать Процесс" {
                        create_process = true;
                    } else if *item == "Переименовать" {
                        rename = true;
                    } else if *item == "Добавить в Запрос" {
                        add_metric_request = true;
                    } else if *item == "Добавить в Результат" {
                        add_metric_result = true;
                    } else if *item == "Показатели" {
                        open_metrics = true;
                    }
                }

                ui.allocate_space(item_rect.size());
            }
        });

    if create_department {
        if let Some(pos) = app.right_click {
            let (cx, cy) = app.canvas.screen_to_cell(pos);
            let existing_ids: Vec<String> = app.canvas.objects.iter().map(|o| o.id.clone()).collect();
            let id = allocate_id(&existing_ids);
            let name = format!("Отдел {}", app.engine.hierarchy().object_count() + 1);
            app.apply_cmd(polygram_core::Command::AddObject {
                id: Some(id.clone()),
                name,
                kind: polygram_core::types::NodeKind::Department,
                at: Some([cx, cy]),
                parent: None,
                direction: None,
                author: None,
            });
        }
        app.right_click = None;
    }

    if create_process {
        if let Some(pos) = app.right_click {
            let (cx, cy) = app.canvas.screen_to_cell(pos);
            let existing_ids: Vec<String> = app.canvas.objects.iter().map(|o| o.id.clone()).collect();
            let id = allocate_id(&existing_ids);
            let name = format!("Процесс {}", app.engine.hierarchy().object_count() + 1);
            app.apply_cmd(polygram_core::Command::AddObject {
                id: Some(id.clone()),
                name,
                kind: polygram_core::types::NodeKind::Process,
                at: Some([cx, cy]),
                parent: None,
                direction: None,
                author: None,
            });
        }
        app.right_click = None;
    }

    if rename {
        if let Some(id) = &app.right_click_object {
            let current_name = app
                .canvas
                .objects
                .iter()
                .find(|o| o.id == *id)
                .map(|o| o.name.clone())
                .unwrap_or_default();
            let is_process = app
                .canvas
                .objects
                .iter()
                .any(|o| o.id == *id && o.kind == polygram_core::types::NodeKind::Process);
            if is_process {
                app.dialogs.edit_target = Some(id.clone());
                app.dialogs.edit_input = current_name;
            } else {
                app.rename_target = Some(id.clone());
                app.rename_input = current_name;
            }
        }
        app.right_click = None;
    }

    if add_metric_request {
        if let Some(id) = &app.right_click_object {
            let n = app
                .canvas
                .objects
                .iter()
                .find(|o| o.id == *id)
                .map(|o| o.requests.len())
                .unwrap_or(0);
            app.dialogs.add_metric_defaults(id.clone(), polygram_core::types::Direction::Request, n);
        }
        app.right_click = None;
    }

    if add_metric_result {
        if let Some(id) = &app.right_click_object {
            let n = app
                .canvas
                .objects
                .iter()
                .find(|o| o.id == *id)
                .map(|o| o.results.len())
                .unwrap_or(0);
            app.dialogs.add_metric_defaults(id.clone(), polygram_core::types::Direction::Result, n);
        }
        app.right_click = None;
    }

    if open_metrics {
        if let Some(id) = &app.right_click_object {
            app.dialogs.container_menu_target = Some(id.clone());
            container_menu::handle_container_menu_dialog(app, published, theme);
        }
        app.right_click = None;
    }

    // Обработка диалога добавления показателя (модальное окно).
    // Вызывается один раз за кадр в主 цикле UI (см. app/mod.rs).
    // if app.dialogs.add_metric_target.is_some() {
    //     add_metric::handle_add_metric_dialog(app, ctx, published);
    // }

    // Закрыть меню по клику вне него.
    if !app.menu_just_opened && ctx.input(|i| i.pointer.any_click()) {
        let pointer_pos = ctx.input(|i| i.pointer.hover_pos());
        let clicked_in_menu = pointer_pos.map_or(false, |p| {
            item_rects.iter().any(|r| r.contains(p))
        });
        if !clicked_in_menu {
            close = true;
        }
    }

    // Закрыть меню по нажатию любой клавиши.
    if !app.menu_just_opened
        && ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Key { pressed: true, .. })))
    {
        close = true;
    }

    if need_clamp {
        app.right_click = Some(clamped_pos);
    }

    if close {
        app.right_click = None;
    }

    app.menu_just_opened = false;
}

/// Диалог переименования отдела (модальное окно).
pub fn rename_dialog(app: &mut super::PolygramApp, ctx: &egui::Context) {
    let rename_target = app.rename_target.clone();
    let mut do_rename = false;
    if let Some(target) = &rename_target {
        egui::Window::new("Переименовать отдел")
            .default_pos(egui::pos2(400.0, 300.0))
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label(format!("Отдел: {target}"));
                ui.text_edit_singleline(&mut app.rename_input);
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        do_rename = true;
                    }
                    if ui.button("Отмена").clicked() {
                        app.rename_target = None;
                        app.rename_input.clear();
                    }
                });
            });
    }
    if do_rename && !app.rename_input.is_empty() {
        if let Some(target) = &app.rename_target {
            if let Some(obj) = app.canvas.objects.iter_mut().find(|o| o.id == *target) {
                obj.name = app.rename_input.clone();
            }
        }
        app.rename_target = None;
        app.rename_input.clear();
    }
}

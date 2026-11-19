//! Контекстное меню по правой кнопке мыши на холсте.
//! Заглушка: пункты «Создать Процесс» и «Создать Отдел».
//! Все пункты одинаковой ширины, вычисляемой по самому длинному пункту.
//! Текст и иконки внутри пунктов выровнены по левому краю по горизонтали,
//! по центру по вертикали.
//! Меню закрывается по клику вне него или по нажатию любой клавиши.
//! Позиция меню ограничена границами экрана.

use egui::{Color32, Context, Rect, Stroke, TextStyle, Window};
use crate::app::icons::paint_lucide_image;
use egui_lucide::Lucide;

/// Контекстное меню по правой кнопке на пустом месте холста.
/// Показывает заглушки «Создать Процесс» и «Создать Отдел» и закрывается по клику на них.
pub fn context_menu(app: &mut super::PolygramApp, ctx: &Context) {
    let Some(pos) = app.right_click else {
        return;
    };

    let items = [
        ("Создать Процесс", Lucide::SquareEqual),
        ("Создать Отдел", Lucide::Group),
    ];
    let mut close = false;

    // Скопировать значения темы, которые нужны внутри замыкания и после него.
    let theme = app.canvas.theme;

    let mut item_rects: Vec<Rect> = Vec::with_capacity(items.len());
    let mut need_clamp = false;
    let mut clamped_pos = pos;

    Window::new("")
        .title_bar(false)
        .default_pos(pos)
        .resizable(false)
        .collapsible(false)
        .frame(
            egui::Frame::new()
                .fill(theme.menu_frame_fill)
                .stroke(Stroke::new(
                    theme.menu_frame_stroke_width,
                    theme.menu_frame_stroke_color,
                ))
                .corner_radius(theme.menu_frame_rounding)
                .inner_margin(theme.menu_frame_margin)
                .shadow(theme.shadow),
        )
        .show(ctx, |ui| {
            // Стиль пунктов меню: нет контура, фон из темы.
            let style = ui.style();
            let font_id = TextStyle::Button.resolve(style);
            let button_padding = style.spacing.button_padding;
            let item_rounding = theme.menu_item_rounding;
            let item_bg_fill = theme.menu_item_bg_fill;
            let hovered_bg_fill = Color32::from_rgb(52, 52, 52);
            let hover_pos = ctx.input(|i| i.pointer.hover_pos());

            let icon_size = 16.0;
            let icon_gap = 8.0;

            // Измерить ширину самого длинного текста.
            let max_text_width = ui.fonts_mut(|f| {
                items
                    .iter()
                    .map(|(item, _)| {
                        let galley = f.layout_no_wrap(item.to_string(), font_id.clone(), Color32::WHITE);
                        galley.size().x
                    })
                    .fold(0.0, f32::max)
            });
            let max_width = icon_size + icon_gap + max_text_width + 2.0 * button_padding.x;
            let item_height = (font_id.size + 2.0 * button_padding.y) * 1.5;
            let menu_height = item_height * items.len() as f32;

            // Ограничить позицию, чтобы меню не уходило за край экрана.
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

            for (item, icon) in items {
                // Рассчитать прямоугольник пункта заранее.
                let cursor = ui.cursor();
                let item_rect = egui::Rect::from_min_size(
                    cursor.min,
                    egui::vec2(max_width, item_height),
                );
                item_rects.push(item_rect);

                // Фон пункта: базовый или при наведении.
                let is_hovered = hover_pos.map_or(false, |p| item_rect.contains(p));
                let bg = if is_hovered {
                    hovered_bg_fill
                } else {
                    item_bg_fill
                };
                ui.painter().rect_filled(item_rect, item_rounding, bg);

                // Интеракция по всему прямоугольнику пункта.
                let response = ui.interact(item_rect, ui.scope_id().with(item), egui::Sense::click());

                // Рисуем иконку Lucide слева от текста (векторный SVG), по центру по вертикали.
                let icon_y_rel = (item_height - icon_size) / 2.0;
                let icon_rect = egui::Rect::from_min_size(
                    item_rect.min + egui::vec2(button_padding.x, icon_y_rel),
                    egui::vec2(icon_size, icon_size),
                );
                paint_lucide_image(ui, icon_rect, icon, Color32::WHITE);

                // Рисуем текст по левому краю после иконки, по центру по вертикали.
                let text_color = Color32::WHITE;
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
                }

                // Продвинуть курсор вниз, чтобы следующий пункт не накладывался.
                ui.allocate_space(item_rect.size());
            }
        });

    // Закрыть меню по клику вне него.
    if ctx.input(|i| i.pointer.any_click()) {
        let pointer_pos = ctx.input(|i| i.pointer.hover_pos());
        let clicked_in_menu = pointer_pos.map_or(false, |p| {
            item_rects.iter().any(|r| r.contains(p))
        });
        if !clicked_in_menu {
            close = true;
        }
    }

    // Закрыть меню по нажатию любой клавиши.
    if ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Key { pressed: true, .. }))) {
        close = true;
    }

    // Применить скорректированную позицию для следующего кадра.
    if need_clamp {
        app.right_click = Some(clamped_pos);
    }

    if close {
        app.right_click = None;
    }
}

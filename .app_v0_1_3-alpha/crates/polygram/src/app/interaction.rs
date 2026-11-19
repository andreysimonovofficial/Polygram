//! Контекстное меню по правой кнопке мыши на холсте.
//! Заглушка: пункты «Создать Процесс» и «Создать Отдел».
//! Все пункты одинаковой ширины, вычисляемой по самому длинному пункту.
//! Текст внутри пунктов выровнен по левому краю.

use egui::{Color32, Context, Stroke, TextStyle, Window};

/// Контекстное меню по правой кнопке на пустом месте холста.
/// Показывает заглушки «Создать Процесс» и «Создать Отдел» и закрывается по клику на них.
pub fn context_menu(app: &mut super::PolygramApp, ctx: &Context) {
    let Some(pos) = app.right_click else {
        return;
    };

    let items = ["Создать Процесс", "Создать Отдел"];
    let mut close = false;

    let theme = &app.canvas.theme;

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
            let (font_id, button_padding) = {
                let style = ui.style_mut();
                style.visuals.widgets.inactive.bg_fill = theme.menu_item_bg_fill;
                style.visuals.widgets.inactive.bg_stroke = Stroke::new(0.0, Color32::TRANSPARENT);
                style.visuals.widgets.inactive.corner_radius = theme.menu_item_rounding;
                (
                    TextStyle::Button.resolve(style),
                    style.spacing.button_padding,
                )
            };
            let item_rounding = theme.menu_item_rounding;
            let item_bg_fill = theme.menu_item_bg_fill;
            let hovered_bg_fill = Color32::from_rgb(60, 60, 70);
            let hover_pos = ctx.input(|i| i.pointer.hover_pos());

            // Измерить ширину самого длинного пункта, чтобы все пункты были одинаковой ширины.
            let max_text_width = ui.fonts_mut(|f| {
                items
                    .iter()
                    .map(|item| {
                        let galley = f.layout_no_wrap(item.to_string(), font_id.clone(), Color32::WHITE);
                        galley.size().x
                    })
                    .fold(0.0, f32::max)
            });
            let max_width = max_text_width + 2.0 * button_padding.x;

            for item in items {
                // Рассчитать прямоугольник пункта заранее, чтобы нарисовать фон под текстом.
                let cursor = ui.cursor();
                let item_rect = egui::Rect::from_min_size(
                    cursor.min,
                    egui::vec2(max_width, font_id.size + 2.0 * button_padding.y),
                );

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

                // Текст пункта по левому краю (рисуем вручную для точного контроля выравнивания).
                let text_color = Color32::WHITE;
                let galley = ui.fonts_mut(|f| {
                    f.layout_no_wrap(item.to_string(), font_id.clone(), text_color)
                });
                let text_pos = item_rect.min + egui::vec2(button_padding.x, button_padding.y);
                ui.painter().galley(text_pos, galley, text_color);

                if response.clicked() {
                    close = true;
                }

                // Продвинуть курсор вниз, чтобы следующий пункт не накладывался на этот.
                ui.allocate_space(item_rect.size());
            }
        });

    if close {
        app.right_click = None;
    }
}

//! Контекстное меню по правой кнопке мыши на холсте.
//! Заглушка: пункты «Создать Процесс» и «Создать Отдел».
//! Все пункты одинаковой ширины, вычисляемой по самому длинному пункту.

use egui::{Color32, Context, Stroke, TextStyle, Window};

/// Контекстное меню по правой кнопке на пустом месте холста.
/// Показывает заглушки «Создать Процесс» и «Создать Отдел» и закрывается по клику на них.
pub fn context_menu(app: &mut super::PolygramApp, ctx: &Context) {
    let Some(pos) = app.right_click else {
        return;
    };

    let items = ["Создать Процесс", "Создать Отдел"];
    let mut close = false;

    Window::new("")
        .title_bar(false)
        .default_pos(pos)
        .resizable(false)
        .collapsible(false)
        .frame(
            egui::Frame::new()
                .fill(app.canvas.theme.bg)
                .stroke(Stroke::new(1.0, Color32::from_rgb(80, 80, 80)))
                .corner_radius(app.canvas.theme.menu_frame_rounding)
                .inner_margin(app.canvas.theme.menu_frame_margin)
                .shadow(app.canvas.theme.shadow),
        )
        .show(ctx, |ui| {
            // Пункты меню заливаются серым при наведении, как в типовом контекстном меню.
            let style = ui.style_mut();
            style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(60, 60, 70);
            style.visuals.widgets.active.bg_fill = Color32::from_rgb(90, 90, 100);
            style.visuals.widgets.hovered.corner_radius =
                app.canvas.theme.menu_item_rounding;
            style.visuals.widgets.active.corner_radius =
                app.canvas.theme.menu_item_rounding;
            style.visuals.widgets.inactive.corner_radius =
                app.canvas.theme.menu_item_rounding;

            // Измерить ширину самого длинного пункта, чтобы все пункты были одинаковой ширины.
            let font_id = TextStyle::Button.resolve(style);
            let button_padding = style.spacing.button_padding;
            let max_text_width = ui.fonts_mut(|f| {
                items
                    .iter()
                    .map(|item| {
                        let galley = f.layout_no_wrap(item.to_string(), font_id.clone(), Color32::BLACK);
                        galley.size().x
                    })
                    .fold(0.0, f32::max)
            });
            let max_width = max_text_width + 2.0 * button_padding.x;

            for item in items {
                if ui
                    .add_sized([max_width, 0.0], egui::Button::new(item))
                    .clicked()
                {
                    close = true;
                }
            }
        });

    if close {
        app.right_click = None;
    }
}

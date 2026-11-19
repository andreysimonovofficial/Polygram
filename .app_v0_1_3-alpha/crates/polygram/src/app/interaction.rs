//! Контекстное меню по правой кнопке мыши на холсте.
//! Заглушка: пункты «Создать Процесс» и «Создать Отдел».

use egui::{Color32, Context, Stroke, Window};

/// Контекстное меню по правой кнопке на пустом месте холста.
/// Показывает заглушки «Создать Процесс» и «Создать Отдел» и закрывается по клику на них.
pub fn context_menu(app: &mut super::PolygramApp, ctx: &Context) {
    let Some(pos) = app.right_click else {
        return;
    };

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
                .shadow(app.canvas.theme.shadow),
        )
        .show(ctx, |ui| {
            // Пункты меню заливаются серым при наведении, как в типовом контекстном меню.
            let style = ui.style_mut();
            style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(60, 60, 70);
            style.visuals.widgets.active.bg_fill = Color32::from_rgb(90, 90, 100);

            if ui.button("Создать Процесс").clicked() {
                close = true;
            }
            if ui.button("Создать Отдел").clicked() {
                close = true;
            }
        });

    if close {
        app.right_click = None;
    }
}

//! Контекстное меню по правой кнопке мыши на холсте.
//! Заглушка: единственный пункт «Создать Процесс».

use egui::{Context, Window};

/// Контекстное меню по правой кнопке на пустом месте холста.
/// Показывает одну заглушку «Создать Процесс» и закрывается по клику на неё.
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
        .frame(egui::Frame::new().shadow(app.canvas.theme.shadow))
        .show(ctx, |ui| {
            if ui.button("Создать Процесс").clicked() {
                close = true;
            }
        });

    if close {
        app.right_click = None;
    }
}

//! Иконки Lucide для всего приложения Polygram.
//! Централизованный модуль для отрисовки векторных иконок Lucide в любом месте UI.
//! Основное отличие от старого приложения: к URI иконки добавляется расширение `.svg`,
//! чтобы egui корректно распознавал содержимое как SVG и не рисовал красную заглушку.

use egui::{Color32, Rect, Sense, Ui, Vec2};
use egui_lucide::Lucide;

/// Иконка Lucide с визуальной обратной связью при наведении.
/// Цвет ярче, когда курсор над иконкой. Возвращает `Response` с `.clicked()`.
#[allow(dead_code)]
pub fn lucide_icon(ui: &mut Ui, icon: Lucide, size: f32, color: Color32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let hovered = response.hovered();
    let draw_color = if hovered { color.linear_multiply(1.25) } else { color };
    paint_lucide_image(ui, rect, icon, draw_color);
    response
}

/// Кнопка-иконка Lucide (алиас для унификации вызовов). Возвращает `Response` с `.clicked()`.
#[allow(dead_code)]
pub fn lucide_button(ui: &mut Ui, icon: Lucide, size: f32, color: Color32) -> egui::Response {
    lucide_icon(ui, icon, size, color)
}

/// Нарисовать иконку Lucide на экране egui. `rect` задаёт размер и положение.
/// Рисует только, не выделяет место — caller сам управляет layout.
pub fn paint_lucide_image(ui: &Ui, rect: Rect, icon: Lucide, color: Color32) {
    let styled = icon.color(color);
    let uri = format!("{}.svg", styled.uri());
    let image = egui::Image::from_bytes(uri, styled.svg().into_bytes());
    image.paint_at(ui, rect);
}

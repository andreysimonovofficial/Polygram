use egui::{Color32, Ui};
use egui_lucide::{Lucide, LucideIcon};

/// Кнопка-иконка Lucide. Возвращает `Response` с `.clicked()`.
pub fn lucide_button(ui: &mut Ui, icon: Lucide, size: f32, color: Color32) -> egui::Response {
    ui.add(LucideIcon::new(icon).size(size).color(color))
}

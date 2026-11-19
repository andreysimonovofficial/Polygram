use egui::{Color32, Rect, Ui};
use egui_lucide::Lucide;

/// Нарисовать иконку Lucide на экране egui. `rect` задаёт размер и положение.
pub fn paint_lucide(ui: &Ui, rect: Rect, icon: Lucide, color: Color32) {
    let image = icon.color(color).size(rect.width()).image();
    image.paint_at(ui, rect);
}

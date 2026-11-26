use egui::{Color32, Rect, Ui};
use egui_lucide::Lucide;
use polygram_core::types::NodeKind;

/// Нарисовать иконку Lucide на экране egui. `rect` задаёт размер и положение.
pub fn paint_lucide(ui: &Ui, rect: Rect, icon: Lucide, color: Color32) {
    let image = icon.color(color).size(rect.width()).image();
    image.paint_at(ui, rect);
}

/// Вернуть иконку Lucide для обложки объекта.
/// Если `custom` не пусто — использовать его (кеbab-case имя Lucide, например "book-up-2").
/// Иначе — иконка по умолчанию для вида объекта: Процесс → BookUp2, Отдел → GraphHub.
pub fn default_icon_for(kind: NodeKind, custom: &str) -> Lucide {
    if !custom.is_empty() {
        return Lucide::from_name(custom).unwrap_or(Lucide::BookUp2);
    }
    match kind {
        NodeKind::Process => Lucide::BookUp2,
        NodeKind::Department => Lucide::GraphHub,
        _ => Lucide::BookUp2,
    }
}

/// Сопоставить строковое имя варианта Lucide с вариантом enum (kebab-case, например "book-up-2").
pub fn lucide_by_name(name: &str) -> Lucide {
    Lucide::from_name(name).unwrap_or(Lucide::BookUp2)
}

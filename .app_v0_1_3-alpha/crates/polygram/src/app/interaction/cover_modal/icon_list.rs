//! Список всех уникальных иконок Lucide.

use egui_lucide::Lucide;

/// Все уникальные иконки Lucide без дублей.
/// Берём напрямую из `Lucide::ALL` (все 1903 иконки библиотеки).
pub fn all_lucide_icons() -> Vec<(String, Lucide)> {
    Lucide::ALL.iter().copied().map(|icon| (icon.name().to_owned(), icon)).collect()
}

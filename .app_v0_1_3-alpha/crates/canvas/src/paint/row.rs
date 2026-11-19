//! Рисование одной строки показателя и строки "+ Добавить показатель".

use crate::icons::paint_lucide;
use crate::paint::constants::{ROW_BORDER_WIDTH, SYMBOL_X_PAD, TEXT_X_PAD};
use crate::status_dot_color;
use egui::{Painter, Pos2, Rect, Stroke, Vec2};
use egui_lucide::Lucide;
use polygram_core::types::Direction;
use polygram_core::{DynamicState, EntryGeom};

/// Рисовать одну строку показателя: точка сцепления слева и справа, имя, значение справа.
/// Для запроса показывается required, для результата — value.
pub fn paint_metric_row(
    ui: &egui::Ui,
    painter: &Painter,
    rect: Rect,
    entry: &EntryGeom,
    dynamic: &DynamicState,
    zoom: f32,
    theme: &crate::Theme,
    is_hovered: bool,
    dir: Direction,
) {
    let status = dynamic.status[entry.ix as usize];
    let cy = rect.center().y;

    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(ROW_BORDER_WIDTH * zoom, theme.card_border),
        egui::StrokeKind::Middle,
    );

    let dot_color = status_dot_color(status, dir, theme);
    let sym_size = (theme.text_size * zoom).max(10.0 * zoom);
    let (left_icon, right_icon) = match dir {
        Direction::Request => (Lucide::ChevronRight, Lucide::ChevronLeft),
        Direction::Result => (Lucide::ChevronLeft, Lucide::ChevronRight),
    };
    paint_lucide(
        ui,
        Rect::from_center_size(Pos2::new(rect.min.x + SYMBOL_X_PAD * zoom, cy), Vec2::splat(sym_size)),
        left_icon,
        dot_color,
    );
    paint_lucide(
        ui,
        Rect::from_center_size(Pos2::new(rect.max.x - SYMBOL_X_PAD * zoom, cy), Vec2::splat(sym_size)),
        right_icon,
        dot_color,
    );

    let name_color = if is_hovered { theme.text } else { theme.text_muted };
    painter.text(
        Pos2::new(rect.min.x + TEXT_X_PAD * zoom, cy),
        egui::Align2::LEFT_CENTER,
        &entry.name,
        egui::FontId::new(theme.text_size * zoom, egui::FontFamily::Proportional),
        name_color,
    );

    let unit = if entry.unit.is_empty() {
        String::new()
    } else {
        format!(" {}", entry.unit)
    };
    let displayed_value = match dir {
        Direction::Request => dynamic.required[entry.ix as usize].clone(),
        Direction::Result => entry.value.clone(),
    };
    let value_text = format!("{:.0}{}", displayed_value, unit);
    painter.text(
        Pos2::new(rect.max.x - TEXT_X_PAD * zoom, cy),
        egui::Align2::RIGHT_CENTER,
        &value_text,
        egui::FontId::new(theme.text_size * zoom, egui::FontFamily::Proportional),
        name_color,
    );
}

/// Рисовать строку "+ Добавить показатель" (центрированная, кликабельная).
pub fn paint_add_row(
    painter: &Painter,
    rect: Rect,
    zoom: f32,
    theme: &crate::Theme,
    is_hovered: bool,
) {
    let add_text = "+ Добавить показатель";
    let add_color = if is_hovered { theme.text } else { theme.text_muted };
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        add_text,
        egui::FontId::new(theme.text_size * zoom, egui::FontFamily::Proportional),
        add_color,
    );
}

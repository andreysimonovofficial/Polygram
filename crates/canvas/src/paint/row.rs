//! Рисование одной строки показателя и строки "+ Добавить показатель".

use egui::{Painter, Pos2, Rect, Stroke, Vec2};
use polygram_core::types::Direction;
use polygram_core::{DynamicState, EntryGeom};

use crate::paint::constants::{ROW_BORDER_WIDTH, SYMBOL_X_PAD, TEXT_X_PAD};
use crate::status_dot_color;

/// Информация о API-привязке одной строки показателя (показывается внутри строки).
#[derive(Clone, Debug, Default)]
pub struct MetricBinding {
    pub ok: bool,
    pub last_updated: Option<String>,
    pub error: Option<String>,
}

/// Рисовать одну строку показателя: точка сцепления слева и справа, имя, значение справа.
/// Для запроса показывается required, для выхода — value.
/// binding — опциональная API-привязка этой строки (иконка + время последнего опроса внутри строки).
/// error_hotspots — накопление rect иконок ошибок + текст ошибки для копирования по клику.
pub fn paint_metric_row(
    painter: &Painter,
    rect: Rect,
    entry: &EntryGeom,
    dynamic: &DynamicState,
    zoom: f32,
    theme: &crate::theme::Theme,
    is_hovered: bool,
    dir: Direction,
    binding: Option<&MetricBinding>,
    error_hotspots: &mut Vec<(Rect, String)>,
) {
    let status = dynamic.status[entry.ix as usize];
    let cy = rect.center().y;

    // Контур всей строки
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(ROW_BORDER_WIDTH * zoom, theme.card_border),
    );

    // Символы сцепления слева и справа внутри тела строки.
    // Запрос: слева ▶, справа ◀. Выход: слева ◀, справа ▶.
    let dot_color = status_dot_color(status, dir, theme);
    let symbol_font = egui::FontId::new((theme.text_size * zoom).max(10.0 * zoom), egui::FontFamily::Proportional);
    let (left_sym, right_sym) = match dir {
        Direction::Request => ("▶", "◀"),
        Direction::Output => ("◀", "▶"),
    };
    painter.text(
        Pos2::new(rect.min.x + SYMBOL_X_PAD * zoom, cy),
        egui::Align2::LEFT_CENTER,
        left_sym,
        symbol_font.clone(),
        dot_color,
    );
    painter.text(
        Pos2::new(rect.max.x - SYMBOL_X_PAD * zoom, cy),
        egui::Align2::RIGHT_CENTER,
        right_sym,
        symbol_font,
        dot_color,
    );

    // Имя записи
    let name_color = if is_hovered { theme.text } else { theme.text_muted };
    painter.text(
        Pos2::new(rect.min.x + TEXT_X_PAD * zoom, cy),
        egui::Align2::LEFT_CENTER,
        &entry.name,
        egui::FontId::new(theme.text_size * zoom, egui::FontFamily::Proportional),
        name_color,
    );

    // Иконка привязки + время последнего опроса (внутри строки, масштабируются с zoom).
    if let Some(b) = binding {
        let icon_color = if b.ok {
            egui::Color32::from_rgb(0, 200, 80)
        } else {
            egui::Color32::from_rgb(240, 80, 80)
        };
        let icon_text = if b.ok { "🌐" } else { "⚠" };
        let name_w_est = entry.name.len() as f32 * 7.0 * zoom;
        let icon_x = rect.min.x + TEXT_X_PAD * zoom + name_w_est + 8.0 * zoom;
        painter.text(
            Pos2::new(icon_x, cy),
            egui::Align2::LEFT_CENTER,
            icon_text,
            egui::FontId::new(theme.text_size * zoom, egui::FontFamily::Proportional),
            icon_color,
        );
        if let Some(ts) = &b.last_updated {
            painter.text(
                Pos2::new(icon_x + 14.0 * zoom, cy),
                egui::Align2::LEFT_CENTER,
                ts,
                egui::FontId::new(theme.text_size * zoom, egui::FontFamily::Proportional),
                egui::Color32::from_gray(170),
            );
        }
        // Кликабельная зона иконки ошибки — копировать текст ошибки по клику.
        if let Some(e) = &b.error {
            if !b.ok {
                let icon_rect = Rect::from_min_size(
                    Pos2::new(icon_x - 2.0 * zoom, cy - 12.0 * zoom),
                    Vec2::new(28.0 * zoom, 24.0 * zoom),
                );
                error_hotspots.push((icon_rect, e.clone()));
            }
        }
    }

    // Значение (справа): для запроса — required, для выхода — value.
    let unit = if entry.unit.is_empty() {
        String::new()
    } else {
        format!(" {}", entry.unit)
    };
    let displayed_value = match dir {
        Direction::Request => dynamic.required[entry.ix as usize].clone(),
        Direction::Output => entry.value.clone(),
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

/// Рисовать графическую копию строки показателя в произвольном месте (превью при Drag&Drop).
pub fn paint_drag_preview(
    painter: &Painter,
    rect: Rect,
    entry: &EntryGeom,
    dynamic: &DynamicState,
    zoom: f32,
    theme: &crate::theme::Theme,
    dir: Direction,
    binding: Option<&MetricBinding>,
) {
    paint_metric_row(painter, rect, entry, dynamic, zoom, theme, false, dir, binding, &mut Vec::new());
}

/// Рисовать строку "+ Добавить показатель" (центрированная, кликабельная).
pub fn paint_add_row(
    painter: &Painter,
    rect: Rect,
    zoom: f32,
    theme: &crate::theme::Theme,
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

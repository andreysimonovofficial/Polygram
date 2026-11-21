//! Рисование рамки отдела по CSS спецификации.
//! Отдел: background #1E1E1E, border #373839, border-radius 8px, padding 8px.
//! Шапка: высота 80px, иконка graph-hub, шрифт 22px.

use crate::theme::Theme;
use crate::icons::paint_lucide;
use egui::{Painter, Pos2, Rect, Stroke, Vec2, Align2, FontId, FontFamily, StrokeKind, Color32};
use polygram_core::ObjectGeom;
use egui_lucide::Lucide;

/// Нарисовать рамку отдела по CSS спецификации.
pub fn paint_department_frame(
    painter: &Painter,
    ui: &egui::Ui,
    object: &ObjectGeom,
    rect_px: Rect,
    zoom: f32,
    theme: &Theme,
    _is_highlighted: bool,
    process_count: usize,
) {
    // CSS значения (масштабируются под zoom).
    let corner_radius = 8.0 * zoom;
    let border_width = 1.0 * zoom;
    let header_h = 80.0 * zoom;
    let icon_size = 16.0 * zoom;
    let padding = 8.0 * zoom;
    let gap = 8.0 * zoom;

    // Фон отдела #1E1E1E (всегда, без переопределения при наведении).
    painter.rect_filled(rect_px, corner_radius, Color32::from_rgb(30, 30, 30));

    // Контур отдела #373839 (всегда, без переопределения при наведении).
    painter.rect_stroke(
        rect_px,
        corner_radius,
        Stroke::new(border_width, Color32::from_rgb(55, 56, 57)),
        StrokeKind::Middle,
    );

    // === Шапка отдела ===
    let header_cy = rect_px.min.y + header_h / 2.0;

    // Иконка типа отдела (graph-hub).
    paint_lucide(
        ui,
        Rect::from_center_size(Pos2::new(rect_px.min.x + padding + icon_size / 2.0, header_cy), Vec2::splat(icon_size)),
        Lucide::GraphHub,
        theme.text,
    );

    // Название отдела: "{Имя} ({Кол. проц})".
    let title = format!("'{}' ({})", object.name, process_count);
    let title_x = rect_px.min.x + padding + icon_size + gap;
    painter.text(
        Pos2::new(title_x, header_cy),
        Align2::LEFT_CENTER,
        &title,
        FontId::new(22.0 * zoom, FontFamily::Proportional),
        theme.text,
    );

    // #ID отдела (справа).
    let id_text = format!("#{}", object.id);
    let id_right = rect_px.max.x - padding - icon_size - gap;
    painter.text(
        Pos2::new(id_right, header_cy),
        Align2::RIGHT_CENTER,
        &id_text,
        FontId::new(22.0 * zoom, FontFamily::Proportional),
        theme.text_muted,
    );
}

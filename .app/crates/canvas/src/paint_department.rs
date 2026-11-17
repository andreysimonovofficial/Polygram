use crate::theme::Theme;
use egui::{Painter, Pos2, Rect, Stroke, Vec2};
use polygram_core::ObjectGeom;
use polygram_core::types::StatusCode;

/// Нарисовать рамку отдела: заливка фона по агрегированному статусу + внешняя граница + шапка с названием.
pub fn paint_department_frame(
    painter: &Painter,
    object: &ObjectGeom,
    rect_px: Rect,
    cell_px: f32,
    zoom: f32,
    theme: &Theme,
    status: StatusCode,
) {
    // Заливка фона отдела цветом его агрегированного статуса.
    // Цвета темнее линий связей, чтобы линии оставались различимыми.
    painter.rect_filled(rect_px, theme.object_corner_radius * zoom, theme.status_fill_color(status));

    // Контур отдела (аура) — только обводка поверх заливки.
    painter.rect_stroke(rect_px, theme.object_corner_radius * zoom, Stroke::new(theme.object_border_width * zoom, theme.card_border), egui::StrokeKind::Middle);

    // Шапка отдела — только текст, без заливки фона.
    let header_h = cell_px * theme.object_header_height_cells;
    let title = format!("Отдел '{}'", object.name);
    painter.text(
        rect_px.min + Vec2::new(12.0 * zoom, header_h / 2.0),
        egui::Align2::LEFT_CENTER,
        &title,
        egui::FontId::new(theme.text_size * zoom, egui::FontFamily::Proportional),
        theme.text,
    );

    let id_text = format!("#{}", object.ix);
    painter.text(
        Pos2::new(rect_px.max.x - 8.0 * zoom, rect_px.min.y + header_h / 2.0),
        egui::Align2::RIGHT_CENTER,
        &id_text,
        egui::FontId::new(theme.text_size * zoom, egui::FontFamily::Proportional),
        theme.text_muted,
    );
}

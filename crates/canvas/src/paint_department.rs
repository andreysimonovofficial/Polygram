use crate::theme::Theme;
use egui::{Painter, Rect, Stroke, Vec2};
use polygram_core::ObjectGeom;

/// Нарисовать рамку отдела: внешняя граница + шапка с названием.
pub fn paint_department_frame(
    painter: &Painter,
    object: &ObjectGeom,
    rect_px: Rect,
    cell_px: f32,
    theme: &Theme,
) {
    // Внешняя рамка отдела
    painter.rect_stroke(
        rect_px,
        2.0,
        Stroke::new(2.0_f32, theme.card_border),
    );
    painter.rect_filled(rect_px, 2.0, theme.card_bg);

    // Шапка отдела
    let header_h = cell_px * 1.2;
    let header_rect = Rect::from_min_size(rect_px.min, Vec2::new(rect_px.width(), header_h));
    painter.rect_filled(header_rect, 0.0, theme.card_header);
    painter.rect_stroke(
        header_rect,
        0.0,
        Stroke::new(1.0_f32, theme.card_border),
    );

    let title = format!("Отдел '{}'", object.name);
    painter.text(
        rect_px.min + Vec2::new(12.0, header_h / 2.0),
        egui::Align2::LEFT_CENTER,
        &title,
        egui::FontId::default(),
        theme.text,
    );

    let id_text = format!("#{}", object.ix);
    painter.text(
        rect_px.max - Vec2::new(12.0, header_h / 2.0),
        egui::Align2::RIGHT_CENTER,
        &id_text,
        egui::FontId::default(),
        theme.text_muted,
    );
}

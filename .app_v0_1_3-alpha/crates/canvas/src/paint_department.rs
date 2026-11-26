//! Рисование рамки отдела по CSS спецификации.
//! Отдел: background #1E1E1E, border #373839, border-radius 8px, padding 8px.
//! Шапка: высота 80px, иконка graph-hub, шрифт 22px.
//! Обложка (когда не наведено): фон отдела + центрированная иконка 10 клеток.

use crate::icons::{default_icon_for, paint_lucide};
use crate::theme::Theme;
use egui::{Painter, Pos2, Rect, Stroke, Vec2, Align2, FontId, FontFamily, StrokeKind, Color32};
use polygram_core::ObjectGeom;
use egui_lucide::Lucide;

/// Нарисовать рамку отдела по CSS спецификации.
/// Когда мышь не наведена — отображается обложка (фон + иконка по центру).
/// Когда наведена — обычный заголовок и содержимое.
pub fn paint_department_frame(
    painter: &Painter,
    ui: &egui::Ui,
    object: &ObjectGeom,
    rect_px: Rect,
    zoom: f32,
    theme: &Theme,
    is_hovered: bool,
    process_count: usize,
) {
    // CSS значения (масштабируются под zoom).
    let corner_radius = 8.0 * zoom;
    let border_width = 1.0 * zoom;
    let header_h = 80.0 * zoom;
    let icon_size = 16.0 * zoom;
    let padding = 8.0 * zoom;
    let gap = 8.0 * zoom;

    if !is_hovered {
        // === Обложка отдела ===
        // Фон полностью перекрывает площадь отдела.
        painter.rect_filled(rect_px, corner_radius, theme.object_bg);
        // Контур обложки.
        painter.rect_stroke(
            rect_px,
            corner_radius,
            Stroke::new(border_width, Color32::from_rgb(55, 56, 57)),
            StrokeKind::Middle,
        );
        // Иконка обложки: квадрат 10 клеток, строго по центру, белый цвет.
        let cover_icon_side = 10.0 * theme.cell_size * zoom;
        let cover_icon = default_icon_for(object.kind, &object.icon);
        let cover_icon_rect = Rect::from_center_size(rect_px.center(), Vec2::splat(cover_icon_side));
        paint_lucide(ui, cover_icon_rect, cover_icon, Color32::WHITE);

        // Подпись под иконкой: duas строки — имя объекта и #ID.
        let name_sig = format!("{}", object.name);
        let id_sig = format!("#{}", object.id);
        let name_size = 13.0 * zoom;
        let id_size = 11.0 * zoom;
        let name_color = Color32::WHITE;
        let id_color = Color32::from_rgb(0xA1, 0xA1, 0xAA);
        let sig_w = cover_icon_side * 1.5;
        let name_rect = Rect::from_center_size(
            Pos2::new(rect_px.center().x, rect_px.center().y + cover_icon_side / 2.0 + 8.0 * zoom),
            Vec2::new(sig_w, name_size * 1.5),
        );
        let id_rect = Rect::from_center_size(
            Pos2::new(rect_px.center().x, rect_px.center().y + cover_icon_side / 2.0 + 8.0 * zoom + name_size + 4.0 * zoom),
            Vec2::new(sig_w, id_size * 1.5),
        );
        painter.text(
            name_rect.center(),
            Align2::CENTER_CENTER,
            &name_sig,
            FontId::new(name_size, FontFamily::Name("inter_semibold".into())),
            name_color,
        );
        painter.text(
            id_rect.center(),
            Align2::CENTER_CENTER,
            &id_sig,
            FontId::new(id_size, FontFamily::Name("inter_medium".into())),
            id_color,
        );

        return;
    }

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

    // #ID отдела (справа, до иконок статуса).
    let id_right = rect_px.max.x - padding - 2.0 * icon_size - 2.0 * gap;
    let id_text = format!("#{}", object.id);
    painter.text(
        Pos2::new(id_right, header_cy),
        Align2::RIGHT_CENTER,
        &id_text,
        FontId::new(22.0 * zoom, FontFamily::Proportional),
        theme.text_muted,
    );

    // Иконка Ожидание (clock-exclamation-point) — перед иконкой избытка.
    let surplus_cx = rect_px.max.x - padding - icon_size / 2.0;
    let waiting_cx = surplus_cx - icon_size - gap;
    paint_lucide(
        ui,
        Rect::from_center_size(Pos2::new(waiting_cx, header_cy), Vec2::splat(icon_size)),
        Lucide::Clock,
        theme.text_muted,
    );

    // Иконка Избыток (square-exclamation-point) — крайнюю справа.
    paint_lucide(
        ui,
        Rect::from_center_size(Pos2::new(surplus_cx, header_cy), Vec2::splat(icon_size)),
        Lucide::SquareExclamationPoint,
        theme.text_muted,
    );
}

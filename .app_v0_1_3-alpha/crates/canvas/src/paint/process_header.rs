//! Рисование шапки процесса: иконка, название, #ID, иконки статуса.

use egui::{Align2, FontId, FontFamily, Painter, Pos2, Rect, Vec2};
use polygram_core::ObjectGeom;
use polygram_core::types::StatusCode;
use polygram_core::DynamicState;
use crate::icons::paint_lucide;
use egui_lucide::Lucide;
use crate::theme::Theme;

/// Рисовать шапку процесса: иконку типа, название, #ID, иконки ожидания/избытка.
/// Возвращает вычисленный цвет статуса процесса.
pub fn paint_process_header(
    painter: &Painter,
    ui: &egui::Ui,
    object: &ObjectGeom,
    dynamic: &DynamicState,
    theme: &Theme,
    ox: f32,
    oy: f32,
    header_h: f32,
    card_w: f32,
    zoom: f32,
    padding: f32,
    icon_size: f32,
    gap: f32,
) -> StatusCode {
    let header_cy = oy + header_h / 2.0;

    let process_status = match (
        crate::paint::status::worst_status(&object.requests, dynamic),
        crate::paint::status::worst_status(&object.results, dynamic),
    ) {
        (StatusCode::Expected, _) | (_, StatusCode::Expected) => StatusCode::Expected,
        (StatusCode::Surplus, _) | (_, StatusCode::Surplus) => StatusCode::Surplus,
        _ => StatusCode::Ok,
    };
    let (process_type_color, process_waiting_color, process_surplus_color) = match process_status {
        StatusCode::Expected => (theme.status_deficit, theme.status_deficit, theme.text_muted),
        StatusCode::Surplus => (theme.status_warning, theme.text_muted, theme.status_warning),
        _ => (theme.text_muted, theme.text_muted, theme.text_muted),
    };

    // Иконка типа процесса (book-up-2).
    paint_lucide(
        ui,
        Rect::from_center_size(
            Pos2::new(ox + padding + icon_size / 2.0, header_cy),
            Vec2::splat(icon_size),
        ),
        Lucide::BookUp2,
        process_type_color,
    );

    // Название процесса.
    let title = object.name.clone();
    let title_x = ox + padding + icon_size + 8.0 * zoom;
    painter.text(
        Pos2::new(title_x, header_cy),
        Align2::LEFT_CENTER,
        &title,
        FontId::new(20.0 * zoom, FontFamily::Proportional),
        theme.text,
    );

    // #ID процесса (справа, до иконок статуса).
    let id_right = ox + card_w - padding - 2.0 * icon_size - 2.0 * gap;
    let id_text = format!("#{}", object.id);
    painter.text(
        Pos2::new(id_right, header_cy),
        Align2::RIGHT_CENTER,
        &id_text,
        FontId::new(20.0 * zoom, FontFamily::Proportional),
        theme.text_muted,
    );

    // Иконка Ожидание (clock-exclamation-point) — перед иконкой избытка.
    let surplus_cx = ox + card_w - padding - icon_size / 2.0;
    let waiting_cx = surplus_cx - icon_size - gap;
    paint_lucide(
        ui,
        Rect::from_center_size(Pos2::new(waiting_cx, header_cy), Vec2::splat(icon_size)),
        Lucide::Clock,
        process_waiting_color,
    );

    // Иконка Избыток (square-exclamation-point) — крайнюю справа.
    paint_lucide(
        ui,
        Rect::from_center_size(Pos2::new(surplus_cx, header_cy), Vec2::splat(icon_size)),
        Lucide::SquareExclamationPoint,
        process_surplus_color,
    );

    process_status
}

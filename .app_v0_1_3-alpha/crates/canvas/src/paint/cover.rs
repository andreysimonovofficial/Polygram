//! Рисование обложки процесса (когда не наведено).

use egui::{Align2, Color32, FontId, FontFamily, Painter, Pos2, Rect, Vec2};
use polygram_core::ObjectGeom;
use crate::icons::{default_icon_for, paint_lucide};
use crate::paint::constants::COVER_CONTENT_SCALE;

/// Нарисовать обложку процесса (фон + иконка + название + плашка #ID).
/// Возвращает `true`, если обложка была нарисована (т.е. процесс не наведён).
pub fn paint_process_cover(
    painter: &Painter,
    ui: &egui::Ui,
    object: &ObjectGeom,
    card_rect: Rect,
    process_radius: f32,
    zoom: f32,
    theme: &crate::Theme,
    is_top_overlap: bool,
) -> bool {
    let is_hovered = false; // cover is only painted when not hovered
    let _ = is_hovered;

    // Фон полностью перекрывает площадь процесса.
    painter.rect_filled(card_rect, process_radius, Color32::from_rgb(32, 32, 32));

    // Контур обложки.
    let border_width = 1.0 * zoom;
    painter.rect_stroke(
        card_rect,
        process_radius,
        egui::Stroke::new(border_width, Color32::from_rgb(55, 56, 57)),
        egui::StrokeKind::Middle,
    );

    // Жёлтая пунктирная обводка — самая верхняя карточка при наложении на другую.
    if is_top_overlap {
        crate::paint::object::paint_dashed_rect(
            painter,
            card_rect,
            process_radius,
            Color32::YELLOW,
            2.0 * zoom,
            8.0 * zoom,
            4.0 * zoom,
        );
    }

    // Иконка обложки: 10 клеток / 3, строго по центру, белый цвет.
    // Содержимое обложки увеличено в 2 раза, чтобы быть соразмерным площади карточки.
    let cover_icon_side = (10.0 / 3.0) * (theme.cell_size * zoom) * COVER_CONTENT_SCALE;
    let cover_icon = default_icon_for(object.kind, &object.icon);
    let center = card_rect.center();

    // Имя объекта и #ID — в вертикальный стек с иконкой, весь стек
    // центрируется относительно центра обложки.
    let name_sig = format!("{}", object.name);
    let id_sig = format!("#{}", object.id);
    let name_size = 13.0 * zoom * COVER_CONTENT_SCALE;
    let id_size = 11.0 * zoom * COVER_CONTENT_SCALE;
    let name_color = Color32::WHITE;
    let id_color = Color32::from_rgb(0xA1, 0xA1, 0xAA);
    let gap = 8.0 * zoom * COVER_CONTENT_SCALE;

    let name_font = FontId::new(name_size, FontFamily::Name("inter_semibold".into()));
    let id_font = FontId::new(id_size, FontFamily::Name("inter_medium".into()));
    let pill_h = id_size * 1.4;
    let pill_w = id_sig.len() as f32 * id_size * 0.52 + 8.0 * zoom * COVER_CONTENT_SCALE;
    let pill_w = pill_w.max(id_size * 2.0);

    let stack_h = cover_icon_side + gap + name_size + gap + pill_h;
    let start_y = center.y - stack_h / 2.0;

    // Иконка — сверху стека, по горизонтали по центру обложки.
    let icon_rect = Rect::from_center_size(
        Pos2::new(center.x, start_y + cover_icon_side / 2.0),
        Vec2::splat(cover_icon_side),
    );
    paint_lucide(ui, icon_rect, cover_icon, Color32::WHITE);

    // Название объекта — по центру стека, по горизонтали по центру обложки.
    let name_y = start_y + cover_icon_side + gap + name_size / 2.0;
    painter.text(
        Pos2::new(center.x, name_y),
        Align2::CENTER_CENTER,
        &name_sig,
        name_font,
        name_color,
    );

    // Плашка #ID — снизу стека, по горизонтали по центру обложки.
    let pill_y = start_y + cover_icon_side + gap + name_size + gap + pill_h / 2.0;
    let pill_rect = Rect::from_center_size(
        Pos2::new(center.x, pill_y),
        Vec2::new(pill_w, pill_h),
    );
    let pill_radius = 2.0 * zoom * COVER_CONTENT_SCALE;
    let pill_border = egui::Stroke::new(1.0 * zoom * COVER_CONTENT_SCALE, Color32::from_rgb(0x37, 0x38, 0x39));
    painter.rect_filled(
        pill_rect,
        pill_radius,
        Color32::from_rgb(0x25, 0x25, 0x25),
    );
    painter.rect_stroke(
        pill_rect,
        pill_radius,
        pill_border,
        egui::StrokeKind::Middle,
    );
    painter.text(
        pill_rect.center(),
        Align2::CENTER_CENTER,
        &id_sig,
        id_font,
        id_color,
    );

    true
}

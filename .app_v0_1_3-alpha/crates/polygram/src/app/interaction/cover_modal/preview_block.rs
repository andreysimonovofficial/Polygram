//! Карточка предпросмотра обложки.

use egui::{Align2, Color32, FontFamily, FontId, Pos2, Rect, Sense, Vec2, Stroke, StrokeKind};
use polygram_core::types::NodeKind;
use polygram_canvas::{default_icon_for, paint_lucide};

/// Карточка предпросмотра обложки: горизонтальная карточка.
/// Слева — иконка в рамке, в центре — текстовый блок, справа — кнопка «Сбросить» на том же уровне.
/// Возвращает `true`, если нажата кнопка «Сбросить».
pub fn preview_block(ui: &mut egui::Ui, icon_name: &str, object_kind: NodeKind) -> bool {
    let card_bg = Color32::from_rgb(0x22, 0x22, 0x22);
    let card_border = Color32::from_rgb(0x37, 0x38, 0x39);

    let available_width = ui.available_width();
    let card_h = 72.0;
    let card_rect = Rect::from_min_size(ui.cursor().min, Vec2::new(available_width, card_h));

    // Всегда выделяем регионы взаимодействия СНАЧАЛА (корректный порядок ID в egui).
    let reset_w = 72.0;
    let reset_h = 30.0;
    let reset_rect = Rect::from_min_size(
        Pos2::new(
            card_rect.max.x - reset_w - 12.0,
            card_rect.center().y - reset_h / 2.0,
        ),
        Vec2::new(reset_w, reset_h),
    );
    let reset_response = ui.allocate_rect(reset_rect, Sense::click());

    // Затем рисуем.
    ui.painter().rect_filled(card_rect, 4.0, card_bg);
    ui.painter().rect_stroke(
        card_rect,
        4.0,
        Stroke::new(1.0, card_border),
        StrokeKind::Middle,
    );

    // Иконка в рамке (40x40, фон #252525, белая иконка 20x20).
    let icon_frame_rect = Rect::from_center_size(
        Pos2::new(card_rect.min.x + 48.0, card_rect.center().y),
        Vec2::splat(40.0),
    );
    ui.painter().rect_filled(icon_frame_rect, 4.0, Color32::from_rgb(0x25, 0x25, 0x25));
    ui.painter().rect_stroke(
        icon_frame_rect,
        4.0,
        Stroke::new(1.0, card_border),
        StrokeKind::Middle,
    );
    let icon = default_icon_for(object_kind, icon_name);
    let icon_rect = Rect::from_center_size(icon_frame_rect.center(), Vec2::splat(20.0));
    paint_lucide(ui, icon_rect, icon, Color32::WHITE);

    // Текстовый блок по центру карточки.
    let text_x = icon_frame_rect.max.x + 16.0;
    let cy = card_rect.center().y;
    let label = match object_kind {
        NodeKind::Process => "Процесс",
        NodeKind::Department => "Отдел",
        _ => "Объект",
    };
    ui.painter().text(
        Pos2::new(text_x, cy - 10.0),
        Align2::LEFT_CENTER,
        label,
        FontId::new(13.0, FontFamily::Name("inter_semibold".into())),
        Color32::WHITE,
    );
    ui.painter().text(
        Pos2::new(text_x, cy + 10.0),
        Align2::LEFT_CENTER,
        "Отображается, когда курсор не наведён",
        FontId::new(11.0, FontFamily::Name("inter_medium".into())),
        Color32::from_rgb(0xA1, 0xA1, 0xAA),
    );

    // Кнопка «Сбросить» справа на том же горизонтальном уровне (внутри карточки).
    let preview_start_y = ui.cursor().min.y;
    let reset_hovered = reset_response.hovered();
    ui.painter().rect_filled(
        reset_rect,
        4.0,
        if reset_hovered {
            Color32::from_rgb(0x28, 0x28, 0x28)
        } else {
            Color32::from_rgb(0x25, 0x25, 0x25)
        },
    );
    ui.painter().rect_stroke(
        reset_rect,
        4.0,
        Stroke::new(1.0, card_border),
        StrokeKind::Middle,
    );
    ui.painter().text(
        reset_rect.center(),
        Align2::CENTER_CENTER,
        "Сбросить",
        FontId::new(12.0, FontFamily::Name("inter_medium".into())),
        Color32::from_rgb(0xD4, 0xD4, 0xD8),
    );

    ui.advance_cursor_after_rect(Rect::from_min_size(
        Pos2::new(card_rect.min.x, preview_start_y),
        Vec2::new(available_width, card_h),
    ));
    ui.add_space(8.0);

    reset_response.clicked()
}

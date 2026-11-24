//! Универсальная оболочка модальных окон Polygram.
//! Источник спецификации:
//! Документация/Продукт/Интерфейс/Сущности/Модальные окна/Модальные окна - Оформление.md
//! Выносит универсальную оболочку модального окна (оверлей, рамка, шапка, подвал,
//! интерактивные состояния кнопок и полей) в общий модуль, отделяя её от
//! специфического тела окна.

use egui::{
    Align, Color32, Context, CornerRadius, Frame, Layout, Margin, PointerButton, Rect, Sense,
    Stroke, StrokeKind, Ui, Vec2,
};
use egui_lucide::Lucide;
use crate::icons::paint_lucide;

// ========== Универсальные параметры модального окна (раздел 1) ==========

/// Фон окна: #1E1E1E.
pub const MODAL_BG: Color32 = Color32::from_rgb(0x1E, 0x1E, 0x1E);
/// Контур (внешняя рамка): 1 px #373839.
pub const MODAL_BORDER: Color32 = Color32::from_rgb(0x37, 0x38, 0x39);
/// Скругление краёв окна: 8 px.
pub const MODAL_RADIUS: u8 = 8;
/// Затемнение фона за окном: 50% (#000000, alpha 50%).
pub const MODAL_OVERLAY_ALPHA: u8 = 128;

// Текст.
/// Цвет основного текста: #FFFFFF.
pub const MODAL_TEXT: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
/// Цвет приглушённого текста: #A1A1AA.
pub const MODAL_TEXT_MUTED: Color32 = Color32::from_rgb(0xA1, 0xA1, 0xAA);

// Интерактивные элементы.
/// Базовый фон интерактивного элемента: #252525.
pub const MODAL_INPUT_BG: Color32 = Color32::from_rgb(0x25, 0x25, 0x25);
/// Фон при наведении/нажатии: #282828.
pub const MODAL_INPUT_HOVER_BG: Color32 = Color32::from_rgb(0x28, 0x28, 0x28);
/// Контур при наведении/нажатии: #4A4A4D.
pub const MODAL_INPUT_HOVER_BORDER: Color32 = Color32::from_rgb(0x4A, 0x4A, 0x4D);

// Кнопка закрытия модального окна (×).
/// Размер кнопки закрытия: 28×28 px.
pub const MODAL_CLOSE_SIZE: f32 = 28.0;
/// Размер иконки крестика: 16×16 px.
pub const MODAL_CLOSE_ICON: f32 = 16.0;
/// Скругление кнопки закрытия: 4 px.
pub const MODAL_CLOSE_RADIUS: u8 = 4;
/// Фон кнопки закрытия при наведении: #252525.
pub const MODAL_CLOSE_HOVER_BG: Color32 = Color32::from_rgb(0x25, 0x25, 0x25);
/// Фон кнопки закрытия при нажатии: #2B2B2B.
pub const MODAL_CLOSE_ACTIVE_BG: Color32 = Color32::from_rgb(0x2B, 0x2B, 0x2B);

// Кнопка подтверждения «Готово».
/// Фиксированная ширина кнопки подтверждения: 89.27 px.
pub const MODAL_CONFIRM_W: f32 = 89.27;
/// Высота кнопки подтверждения: 32 px.
pub const MODAL_CONFIRM_H: f32 = 32.0;
/// Фон кнопки подтверждения: #FFFFFF.
pub const MODAL_CONFIRM_BG: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
/// Фон кнопки подтверждения при наведении: #E5E5E5.
pub const MODAL_CONFIRM_HOVER: Color32 = Color32::from_rgb(0xE5, 0xE5, 0xE5);
/// Фон кнопки подтверждения при нажатии: #D4D4D4.
pub const MODAL_CONFIRM_ACTIVE: Color32 = Color32::from_rgb(0xD4, 0xD4, 0xD4);
/// Рамка кнопки подтверждения: 1 px #FFFFFF.
pub const MODAL_CONFIRM_BORDER: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
/// Цвет текста кнопки подтверждения: #1E1E1E.
pub const MODAL_CONFIRM_TEXT: Color32 = Color32::from_rgb(0x1E, 0x1E, 0x1E);
/// Скругление кнопки подтверждения: 4 px.
pub const MODAL_CONFIRM_RADIUS: u8 = 4;

// Кнопка смены типа показателя (состояния, общие для модалок).
/// Фон кнопки типа при наведении: #2B2B2B.
pub const MODAL_TYPE_HOVER_BG: Color32 = Color32::from_rgb(0x2B, 0x2B, 0x2B);
/// Фон кнопки типа при нажатии: #323232.
pub const MODAL_TYPE_ACTIVE_BG: Color32 = Color32::from_rgb(0x32, 0x32, 0x32);

/// Нарисовать полноэкранный оверлей с затемнением 50% под модальным окном.
pub fn paint_overlay(ctx: &Context) {
    let screen_rect = ctx.viewport_rect();
    ctx.layer_painter(egui::LayerId::background()).rect_filled(
        screen_rect,
        0.0,
        Color32::from_black_alpha(MODAL_OVERLAY_ALPHA),
    );
}

/// Универсальная рамка модального окна.
/// Фон #1E1E1E, контур 1 px #373839, скругление 8 px, без внутреннего отступа.
pub fn modal_frame() -> Frame {
    Frame::new()
        .fill(MODAL_BG)
        .stroke(Stroke::new(1.0, MODAL_BORDER))
        .corner_radius(CornerRadius::same(MODAL_RADIUS))
        .inner_margin(Margin::same(0))
}

/// Универсальная шапка модального окна.
/// Отступы: 20 px сверху, 20 px слева, 20 px справа, 12 px снизу, фон #1E1E1E.
/// Левый текстовый блок: вертикальный стек (заголовок + подзаголовок), зазор 4 px.
/// Кнопка закрытия × (28×28, скругление 4 px) выровнена по правому и верхнему краю.
/// Возвращает `true`, если нажата кнопка закрытия.
pub fn header(ui: &mut Ui, title: &str, subtitle: &str) -> bool {
    let mut closed = false;
    Frame::new()
        .fill(MODAL_BG)
        .inner_margin(Margin {
            left: 20,
            top: 20,
            right: 20,
            bottom: 12,
        })
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(0.0, 0.0);

                // Левый текстовый блок: вертикальный стек, зазор 4 px.
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(0.0, 4.0);
                    ui.label(
                        egui::RichText::new(title)
                            .strong()
                            .size(16.0)
                            .color(MODAL_TEXT),
                    );
                    ui.label(
                        egui::RichText::new(subtitle)
                            .strong()
                            .size(11.0)
                            .extra_letter_spacing(0.44)
                            .color(MODAL_TEXT_MUTED),
                    );
                });

                // Кнопка закрытия выровнена по правому верхнему углу шапки.
                ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                    if close_button(ui) {
                        closed = true;
                    }
                });
            });
        });
    closed
}

/// Кнопка закрытия модального окна (×) с состояниями наведения и нажатия.
/// Базовый цвет иконки #A1A1AA; наведение — фон #252525, иконка #FFFFFF;
/// нажатие — фон #2B2B2B, иконка #FFFFFF.
/// Возвращает `true` при клике.
pub fn close_button(ui: &mut Ui) -> bool {
    let size = Vec2::splat(MODAL_CLOSE_SIZE);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let hovered = response.hovered();
    let pressed = hovered && ui.ctx().input(|i| i.pointer.button_down(PointerButton::Primary));

    let icon_color = if hovered || pressed {
        MODAL_TEXT
    } else {
        MODAL_TEXT_MUTED
    };
    let fill = if pressed {
        MODAL_CLOSE_ACTIVE_BG
    } else if hovered {
        MODAL_CLOSE_HOVER_BG
    } else {
        MODAL_BG
    };

    ui.painter()
        .rect_filled(rect, CornerRadius::same(MODAL_CLOSE_RADIUS), fill);
    let icon_rect =
        Rect::from_center_size(rect.center(), Vec2::splat(MODAL_CLOSE_ICON));
    paint_lucide(ui, icon_rect, Lucide::X, icon_color);

    response.clicked()
}

/// Универсальный подвал модального окна.
/// Высота 52 px, отступы 20/20/20 (слева/справа/снизу), фон #1E1E1E,
/// выравнивание по правому краю и центру по вертикали.
/// Рисует кнопку подтверждения «Готово». Возвращает `true` при клике на кнопку.
pub fn footer(ui: &mut Ui) -> bool {
    let mut confirmed = false;
    Frame::new()
        .fill(MODAL_BG)
        .inner_margin(Margin {
            left: 20,
            top: 0,
            right: 20,
            bottom: 20,
        })
        .show(ui, |ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);
                if confirm_button(ui) {
                    confirmed = true;
                }
            });
        });
    confirmed
}

/// Кнопка подтверждения «Готово» с состояниями наведения и нажатия.
/// Фиксированный текст `Готово` во всех модальных окнах.
/// Фон #FFFFFF, наведение #E5E5E5, нажатие #D4D4D4, рамка 1 px #FFFFFF,
/// скругление 4 px, текст Inter 14 px полужирный #1E1E1E.
/// Возвращает `true` при клике.
pub fn confirm_button(ui: &mut Ui) -> bool {
    let size = Vec2::new(MODAL_CONFIRM_W, MODAL_CONFIRM_H);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let hovered = response.hovered();
    let pressed = hovered && ui.ctx().input(|i| i.pointer.button_down(PointerButton::Primary));

    let fill = if pressed {
        MODAL_CONFIRM_ACTIVE
    } else if hovered {
        MODAL_CONFIRM_HOVER
    } else {
        MODAL_CONFIRM_BG
    };

    ui.painter()
        .rect_filled(rect, CornerRadius::same(MODAL_CONFIRM_RADIUS), fill);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(MODAL_CONFIRM_RADIUS),
        Stroke::new(1.0, MODAL_CONFIRM_BORDER),
        StrokeKind::Middle,
    );

    let font_id = egui::FontId::new(14.0, egui::FontFamily::Proportional);
    let galley = ui
        .fonts_mut(|f| f.layout_no_wrap("Готово".to_owned(), font_id, MODAL_CONFIRM_TEXT));
    let text_pos = rect.center() - galley.size() / 2.0;
    ui.painter().galley(text_pos, galley, MODAL_CONFIRM_TEXT);

    response.clicked()
}

/// Текстовое поле модального окна с состояниями наведения и нажатия.
/// Базовый фон #252525, контур 1 px #373839, скругление 2 px.
/// При наведении/нажатии: фон #282828, контур 1 px #4A4A4D.
/// `padding` — внутренний отступ (например, 10 px по горизонтали и 7.5 px по вертикали).
/// Возвращает ответ текстового поля.
pub fn input_field(
    ui: &mut Ui,
    text: &mut String,
    hint: &str,
    size: Vec2,
    right_align: bool,
    padding: Margin,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    let hovered = response.hovered();

    let fill = if hovered {
        MODAL_INPUT_HOVER_BG
    } else {
        MODAL_INPUT_BG
    };
    let stroke = if hovered {
        MODAL_INPUT_HOVER_BORDER
    } else {
        MODAL_BORDER
    };

    ui.painter()
        .rect_filled(rect, CornerRadius::same(2), fill);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(2),
        Stroke::new(1.0, stroke),
        StrokeKind::Middle,
    );

    let layout = if right_align {
        Layout::right_to_left(Align::Center)
    } else {
        Layout::left_to_right(Align::Center)
    };
    let inner_rect = Rect::from_min_max(
        egui::pos2(
            rect.min.x + padding.left as f32,
            rect.min.y + padding.top as f32,
        ),
        egui::pos2(
            rect.max.x - padding.right as f32,
            rect.max.y - padding.bottom as f32,
        ),
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner_rect)
            .layout(layout),
    );
    child.add(
        egui::TextEdit::singleline(text)
            .frame(
                Frame::new()
                    .fill(Color32::TRANSPARENT)
                    .stroke(Stroke::new(0.0, Color32::TRANSPARENT)),
            )
            .text_color(MODAL_TEXT)
            .hint_text(hint),
    );

    response
}

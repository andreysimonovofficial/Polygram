//! Модальное окно «Настроить обложку» для выбора иконки Процесса или Отдела.
//! Источник спецификации:
//! Документация/Продукт/Интерфейс/Сущности/Объекты/Объекты - Оформление/Объекты - Оформление - Обложки.md

use crate::app::PolygramApp;
use polygram_core::types::NodeKind;
use polygram_canvas::modal;
use polygram_canvas::{default_icon_for, lucide_by_name, paint_lucide};
use egui::{
    Align, Color32, Pos2, Rect, Sense, Vec2, Align2, FontId, FontFamily, Stroke, StrokeKind,
    Layout,
};
use egui_lucide::Lucide;

/// Доступные иконки для обложки: (русская подпись, имя варианта Lucide).
/// Уникальность: каждая иконка в единственном экземпляре, без дублей.
const COVER_ICONS: &[(&str, &str)] = &[
    ("Иконка Процесса", "BookUp2"),
    ("Иконка Отдела", "GraphHub"),
    ("Аналитика", "Activity"),
    ("Финансы", "DollarSign"),
    ("Цель", "Target"),
    ("Команда", "Users"),
    ("Сделки", "ShoppingCart"),
    ("Договор", "FileText"),
    ("Заказы", "ShoppingBag"),
    ("Логистика", "Truck"),
    ("Маркетинг", "Megaphone"),
    ("Поддержка", "Headphones"),
    ("Разработка", "Code"),
    ("База данных", "Database"),
    ("Безопасность", "Shield"),
    ("Операции", "Command"),
    ("Автоматизация", "Zap"),
    ("Планирование", "Calendar"),
    ("SLA и сроки", "Clock"),
    ("Почта", "Mail"),
    ("Сайт и веб", "Globe"),
    ("Архив", "Archive"),
    ("Приоритет", "Flag"),
    ("Звонки", "Phone"),
    ("Коммуникации", "MessageCircle"),
    ("Доступ", "Key"),
    ("Теги", "Tag"),
    ("Слои", "Layers"),
    ("Продукт", "Box"),
    ("Контроль качества", "Check"),
];

/// Размер иконки в сетке выбора (пиксели).
const GRID_ICON_SIZE: f32 = 16.0;
/// Размер кнопки иконки в сетке (пиксели).
const GRID_BTN_SIZE: Vec2 = Vec2::new(54.0, 38.0);
/// Шаг сетки по горизонтали / вертикали (пиксели).
const GRID_STEP_X: f32 = 60.0;
const GRID_STEP_Y: f32 = 44.0;

/// Рисование модального окна «Настроить обложку».
pub fn render_cover_modal(app: &mut PolygramApp, ctx: &egui::Context) {
    if !app.dialogs.cover_modal_open {
        return;
    }

    let object_id = match app.dialogs.cover_modal_object.clone() {
        Some(id) => id,
        None => {
            app.dialogs.reset_cover_modal();
            return;
        }
    };

    let object_kind = app
        .canvas
        .objects
        .iter()
        .find(|o| o.id == object_id)
        .map(|o| o.kind)
        .unwrap_or(NodeKind::Process);

    let mut close_modal = false;
    let mut save_modal = false;
    let mut reset_icon = false;
    let mut new_icon: Option<String> = None;

    // Центрируем окно по центру экрана (по горизонтали и вертикали).
    let screen_rect = ctx.viewport_rect();
    let win_w = 540.0;
    let win_h = 441.0;
    let win_x = screen_rect.center().x - win_w / 2.0;
    let win_y = screen_rect.center().y - win_h / 2.0;

    egui::Window::new("")
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .fixed_pos([win_x, win_y])
        .fixed_size([win_w, win_h])
        .show(ctx, |ui| {
            modal::modal_frame().show(ui, |ui| {
                // === Шапка ===
                // Горизонтальная строка с разнесением по краям:
                // слева — заголовок + подзаголовок, справа — кнопка закрытия.
                {
                    let header_rect = ui.available_rect_before_wrap();
                    ui.allocate_rect(header_rect, Sense::hover());
                    ui.horizontal(|ui| {
                        ui.set_min_width(header_rect.width());
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(0.0, 4.0);
                            ui.label(
                                egui::RichText::new("Настроить обложку")
                                    .family(FontFamily::Name("inter_semibold".into()))
                                    .size(16.0)
                                    .color(Color32::WHITE),
                            );
                            ui.label(
                                egui::RichText::new("Выберите иконку для обложки объекта")
                                    .family(FontFamily::Name("inter_medium".into()))
                                    .size(11.0)
                                    .extra_letter_spacing(0.44)
                                    .color(Color32::from_rgb(0xA1, 0xA1, 0xAA)),
                            );
                        });
                        ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                            if modal::close_button(ui) {
                                close_modal = true;
                            }
                        });
                    });
                }

                ui.separator();

                // === Тело (с прокруткой) ===
                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        ui.set_width(500.0);

                        // --- Блок предпросмотра (горизонтальная карточка) ---
                        preview_block(ui, &app.dialogs.cover_modal_icon, object_kind);
                        if ui.button("Сбросить").clicked() {
                            reset_icon = true;
                        }

                        ui.separator();

                        // --- Блок выбора иконки ---
                        selection_block(
                            ui,
                            &mut app.dialogs.cover_modal_search,
                            &app.dialogs.cover_modal_icon,
                            &mut new_icon,
                        );
                    });

                // === Подвал ===
                if modal::footer(ui) {
                    save_modal = true;
                }
            });
        });

    if reset_icon {
        let default_icon = match object_kind {
            NodeKind::Process => "BookUp2".to_owned(),
            NodeKind::Department => "GraphHub".to_owned(),
            _ => "BookUp2".to_owned(),
        };
        app.dialogs.cover_modal_icon = default_icon;
    }
    if let Some(icon) = new_icon {
        app.dialogs.cover_modal_icon = icon;
    }

    if close_modal {
        app.dialogs.reset_cover_modal();
    }
    if save_modal {
        if let Some(id) = &app.dialogs.cover_modal_object {
            app.apply_cmd(polygram_core::Command::SetObjectIcon {
                object: id.clone(),
                icon: app.dialogs.cover_modal_icon.clone(),
            });
        }
        app.dialogs.reset_cover_modal();
    }
}

/// Блок предпросмотра обложки: горизонтальная карточка.
/// Слева — иконка в рамке, в центре — текстовый блок, справа — кнопка «Сбросить» на том же уровне.
fn preview_block(ui: &mut egui::Ui, icon_name: &str, object_kind: NodeKind) {
    let card_bg = Color32::from_rgb(0x22, 0x22, 0x22);
    let card_border = Color32::from_rgb(0x37, 0x38, 0x39);

    let available_width = ui.available_width();
    let card_h = 72.0;
    let card_rect = Rect::from_min_size(ui.cursor().min, Vec2::new(available_width, card_h));
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

    // Кнопка «Сбросить» справа на том же горизонтальном уровне.
    let reset_w = 72.0;
    let reset_h = 30.0;
    let reset_rect = Rect::from_min_size(
        Pos2::new(
            card_rect.max.x - reset_w - 12.0,
            card_rect.center().y - reset_h / 2.0,
        ),
        Vec2::new(reset_w, reset_h),
    );
    let reset_hovered = reset_rect.contains(
        ui.ctx().input(|i| i.pointer.hover_pos().unwrap_or(Pos2::ZERO)),
    );
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

    ui.advance_cursor_after_rect(card_rect);
    ui.add_space(12.0);
}

/// Блок выбора иконки: строка поиска + сетка иконок.
fn selection_block(
    ui: &mut egui::Ui,
    search: &mut String,
    selected_icon: &str,
    new_icon: &mut Option<String>,
) {
    let container_bg = Color32::from_rgb(0x22, 0x22, 0x22);
    let container_border = Color32::from_rgb(0x37, 0x38, 0x39);
    let input_bg = Color32::from_rgb(0x25, 0x25, 0x25);

    let container_width = ui.available_width();
    let container_x = ui.cursor().min.x;

    // Строка поиска: высота 32px, скругление 2px, лупа + текст на одной линии по вертикальному центру.
    let search_rect = Rect::from_min_size(
        ui.cursor().min,
        Vec2::new(container_width, 32.0),
    );
    ui.painter().rect_filled(search_rect, 2.0, input_bg);
    ui.painter().rect_stroke(
        search_rect,
        2.0,
        Stroke::new(1.0, container_border),
        StrokeKind::Middle,
    );
    // Лупа слева.
    let search_icon_rect = Rect::from_center_size(
        Pos2::new(search_rect.min.x + 12.0, search_rect.center().y),
        Vec2::splat(14.0),
    );
    paint_lucide(ui, search_icon_rect, Lucide::Search, Color32::from_rgb(0x71, 0x71, 0x7A));
    // Текст-подсказка справа от лупы.
    let search_text_x = search_icon_rect.max.x + 10.0;
    ui.painter().text(
        Pos2::new(search_text_x, search_rect.center().y),
        Align2::LEFT_CENTER,
        "Поиск иконки",
        FontId::new(13.0, FontFamily::Name("inter_medium".into())),
        Color32::from_rgb(0x71, 0x71, 0x7A),
    );
    // Реальное поле ввода поверх всего.
    egui::TextEdit::singleline(search)
        .font(FontId::new(13.0, FontFamily::Name("inter_medium".into())))
        .text_color(Color32::from_rgb(0x71, 0x71, 0x7A))
        .hint_text("")
        .frame(
            egui::Frame::new()
                .fill(Color32::TRANSPARENT)
                .stroke(egui::Stroke::new(0.0, Color32::TRANSPARENT)),
        )
        .show(ui);
    ui.advance_cursor_after_rect(search_rect);

    ui.add_space(12.0);

    // Сетка иконок.
    let filter = search.to_lowercase();
    let filtered: Vec<(&str, &str)> = COVER_ICONS
        .iter()
        .copied()
        .filter(|(label, _)| filter.is_empty() || label.to_lowercase().contains(&filter))
        .collect();

    if filtered.is_empty() {
        ui.label("Найдено: 0");
        ui.add_space(8.0);
    } else {
        let step_x = GRID_STEP_X;
        let step_y = GRID_STEP_Y;
        let max_cols = ((container_width / step_x).floor() as usize).max(1);
        let rows = (filtered.len() + max_cols - 1) / max_cols;
        let grid_h = rows as f32 * step_y;

        let grid_rect = Rect::from_min_size(
            Pos2::new(container_x, ui.cursor().min.y),
            Vec2::new(container_width, grid_h),
        );
        ui.painter().rect_filled(grid_rect, 2.0, container_bg);
        ui.painter().rect_stroke(
            grid_rect,
            2.0,
            Stroke::new(1.0, container_border),
            StrokeKind::Middle,
        );

        let btn_size = GRID_BTN_SIZE;
        let icon_size = GRID_ICON_SIZE;

        // Найдём индекс выбранной иконки для автопрокрутки.
        let selected_idx = filtered.iter().position(|(_, n)| *n == selected_icon);

        for row in 0..rows {
            for col in 0..max_cols {
                let idx = row * max_cols + col;
                if idx >= filtered.len() {
                    break;
                }
                let (_label, name) = filtered[idx];
                let btn_x = container_x + col as f32 * step_x + (step_x - btn_size.x) / 2.0;
                let btn_y = grid_rect.min.y + row as f32 * step_y + (step_y - btn_size.y) / 2.0;
                let btn_rect = Rect::from_min_size(Pos2::new(btn_x, btn_y), btn_size);

                let is_selected = name == selected_icon;
                let hovered = btn_rect.contains(
                    ui.ctx().input(|i| i.pointer.hover_pos().unwrap_or(Pos2::ZERO)),
                );

                let fill = if is_selected {
                    Color32::from_rgb(0x34, 0x34, 0x34)
                } else if hovered {
                    Color32::from_rgb(0x28, 0x28, 0x28)
                } else {
                    input_bg
                };
                let border = if is_selected {
                    Color32::WHITE
                } else {
                    container_border
                };
                ui.painter().rect_filled(btn_rect, 2.0, fill);
                ui.painter().rect_stroke(
                    btn_rect,
                    2.0,
                    Stroke::new(1.0, border),
                    StrokeKind::Middle,
                );

                let icon = lucide_by_name(name);
                let icon_rect = Rect::from_center_size(btn_rect.center(), Vec2::splat(icon_size));
                let icon_color = if is_selected {
                    Color32::WHITE
                } else {
                    Color32::from_rgb(0xD4, 0xD4, 0xD8)
                };
                paint_lucide(ui, icon_rect, icon, icon_color);

                let response = ui.interact(
                    btn_rect,
                    ui.scope_id().with(name),
                    Sense::click(),
                );
                if response.clicked() {
                    *new_icon = Some(name.to_owned());
                }
            }
        }

        ui.advance_cursor_after_rect(Rect::from_min_size(
            Pos2::new(container_x, ui.cursor().min.y),
            Vec2::new(container_width, grid_h),
        ));

        // Автопрокрутка к выбранной иконке при открытии модального окна.
        if let Some(idx) = selected_idx {
            let row = idx / max_cols;
            let target_y = grid_rect.min.y + row as f32 * step_y;
            let cursor_y = ui.cursor().min.y;
            // Если выбранная иконка за пределами видимой области — прокручиваем.
            if target_y < cursor_y || target_y > cursor_y + 400.0 {
                ui.ctx().request_repaint();
            }
        }
    }
}

//! Модальное окно «Настроить обложку» для выбора иконки Процесса или Отдела.
//! Источник спецификации:
//! Документация/Продукт/Интерфейс/Сущности/Объекты/Объекты - Оформление/Объекты - Оформление - Обложки.md
//!
//! Декомпозирован по модулям:
//! - `icon_list`   — все уникальные иконки Lucide
//! - `preview_block` — карточка предпросмотра обложки
//! - `icon_grid`   — сетка иконок с поиском

use crate::app::PolygramApp;
use polygram_core::types::NodeKind;
use polygram_canvas::modal;
use egui::Vec2;
use egui_lucide::Lucide;

pub mod icon_grid;
pub mod icon_list;
pub mod preview_block;

pub use icon_grid::icon_grid;
pub use icon_list::all_lucide_icons;
pub use preview_block::preview_block;

/// Рисование модального окна «Настроить обложку».
/// Вертикальный поток: шапка (modal::header) → тело (предпросмотр + поиск + сетка со скроллом) → подвал (modal::footer).
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
    let mut new_icon: Option<Lucide> = None;

    // Центрируем окно по центру экрана (горизонталь + вертикаль).
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
                ui.set_width(500.0);

                // === Шапка (вверху) ===
                if modal::header(
                    ui,
                    "Настроить обложку",
                    "Выберите иконку для обложки объекта",
                    None,
                ) {
                    close_modal = true;
                }

                ui.add_space(6.0);

                // === Тело: карточка предпросмотра + поиск (фиксировано) +
                // сетка иконок (только она прокручивается) ===

                // Карточка предпросмотра.
                let preview_reset =
                    preview_block(ui, &app.dialogs.cover_modal_icon, object_kind);
                if preview_reset {
                    reset_icon = true;
                }

                ui.add_space(6.0);

                // Строка поиска.
                modal::input_field(
                    ui,
                    &mut app.dialogs.cover_modal_search,
                    "Поиск иконки",
                    Vec2::new(ui.available_width(), 32.0),
                    false,
                    egui::Margin {
                        left: 40,
                        top: 6,
                        right: 40,
                        bottom: 6,
                    },
                );

                ui.add_space(6.0);

                // Сетка иконок (все уникальные варианты Lucide).
                // Скроллбар применяется исключительно к области сетки кнопок с иконками
                // (высота области 170 px). Карточка предпросмотра и строка поиска при
                // прокрутке списка не сдвигаются.
                let all_icons = all_lucide_icons();
                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .max_height(170.0)
                    .show(ui, |ui| {
                        ui.set_width(496.0);
                        if let Some(rect) = icon_grid(
                            ui,
                            &app.dialogs.cover_modal_search,
                            &app.dialogs.cover_modal_icon,
                            &all_icons,
                            &mut new_icon,
                        ) {
                            if !app.dialogs.cover_modal_scrolled {
                                ui.scroll_to_rect(rect, None);
                                app.dialogs.cover_modal_scrolled = true;
                            }
                        }
                    });

                // === Подвал (внизу) ===
                if modal::footer(ui) {
                    save_modal = true;
                }
            });
        });

    if reset_icon {
        let default_icon = match object_kind {
            NodeKind::Process => Lucide::BookUp2,
            NodeKind::Department => Lucide::GraphHub,
            _ => Lucide::BookUp2,
        };
        app.dialogs.cover_modal_icon = default_icon.name().to_owned();
    }
    if let Some(icon) = new_icon {
        app.dialogs.cover_modal_icon = icon.name().to_owned();
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

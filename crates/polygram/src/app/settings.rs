//! Окно детальных настроек темы оформления.
//! Цвета задаются ячейкой-образцом рядом с меткой; палитра открывается только при нажатии на ячейку.
//! Показывает все настраиваемые в egui параметры: отступы, скругление, шрифты, цвета,
//! а также параметры холста (скругление карточек, скруглённые углы ортогональных линий).

use crate::app::PolygramApp;
use egui::widgets::color_picker::{color_picker_color32, Alpha};
use polygram_canvas::Theme;

/// Рисовать окно настроек темы. Возвращает `true`, если что-то изменилось (нужно сохранить конфиг).
pub fn render_settings(app: &mut PolygramApp, ui: &mut egui::Ui) -> bool {
    let mut changed = false;
    let mut open = app.show_settings;

    if open {
        let theme = &mut app.canvas.theme;
        let mut close_requested = false;
        let mut save_default = false;

        egui::Window::new("Настройки темы")
            .id(egui::Id::new("theme_settings_window"))
            .open(&mut open)
            .resizable(true)
            .default_size([520.0, 640.0])
            .show(ui.ctx(), |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("Цвета холста");
                    changed |= color(ui, "Фон", &mut theme.bg);
                    changed |= color(ui, "Карточка", &mut theme.card_bg);
                    changed |= color(ui, "Рамка карточки", &mut theme.card_border);
                    changed |= color(ui, "Заголовок", &mut theme.card_header);
                    changed |= color(ui, "Линия связи", &mut theme.line);
                    changed |= color(ui, "Линия (наведение)", &mut theme.line_hover);
                    changed |= color(ui, "Сетка", &mut theme.grid_color);

                    ui.separator();
                    ui.heading("Статусы");
                    changed |= color(ui, "OK", &mut theme.status_ok);
                    changed |= color(ui, "Предупреждение", &mut theme.status_warning);
                    changed |= color(ui, "Дефицит", &mut theme.status_deficit);
                    changed |= color(ui, "Избыток", &mut theme.status_surplus);
                    changed |= color(ui, "Не подключено", &mut theme.status_unconnected);
                    changed |= color(ui, "Недействителен", &mut theme.status_invalid);

                    ui.separator();
                    ui.heading("Текст");
                    changed |= color(ui, "Основной", &mut theme.text);
                    changed |= color(ui, "Приглушённый", &mut theme.text_muted);

                    ui.separator();
                    ui.heading("Оболочка");
                    changed |= color(ui, "Фон панели", &mut theme.panel_bg);
                    changed |= color(ui, "Рамка панели", &mut theme.panel_border);
                    changed |= color(ui, "Текст панели", &mut theme.panel_text);
                    changed |= color(ui, "Фон меню", &mut theme.menu_bg);
                    changed |= color(ui, "Текст меню", &mut theme.menu_text);
                    changed |= color(ui, "Наведение меню", &mut theme.menu_hover);
                    changed |= color(ui, "Строка состояния", &mut theme.status_bar_bg);
                    changed |= color(ui, "Текст состояния", &mut theme.status_bar_text);

                    ui.separator();
                    ui.heading("Акценты");
                    changed |= color(ui, "Акцент", &mut theme.accent);
                    changed |= color(ui, "Акцент 2", &mut theme.accent_secondary);
                    changed |= color(ui, "Выделение", &mut theme.highlight);
                    changed |= color(ui, "Ссылка", &mut theme.link_color);
                    changed |= color(ui, "Слабый фон", &mut theme.faint_bg_color);
                    changed |= color(ui, "Экстр. фон", &mut theme.extreme_bg_color);
                    changed |= color(ui, "Фон кода", &mut theme.code_bg_color);

                    ui.separator();
                    ui.heading("Карточки и линии (холст)");
                    changed |= drag(ui, "Скругление краёв карточек", &mut theme.object_corner_radius, 0.0, 64.0, 0.1);
                    changed |= drag(ui, "Ширина рамки", &mut theme.object_border_width, 0.0, 8.0, 0.1);
                    changed |= drag(ui, "Рамка при наведении", &mut theme.object_border_hover_width, 0.0, 8.0, 0.1);
                    changed |= drag(ui, "Заголовок (клеток)", &mut theme.object_header_height_cells, 0.0, 4.0, 0.1);
                    changed |= drag(ui, "Заголовок контейнера", &mut theme.container_header_height_cells, 0.0, 4.0, 0.1);
                    changed |= drag(ui, "Радиус порта", &mut theme.port_radius, 0.0, 24.0, 0.1);
                    changed |= drag(ui, "Текст (холст)", &mut theme.text_size, 6.0, 72.0, 0.5);
                    changed |= drag(ui, "Ширина линии связи", &mut theme.edge_width, 0.1, 8.0, 0.1);
                    changed |= drag(ui, "Скруглённые углы линий (0 = прямо)", &mut theme.line_corner_radius, 0.0, 48.0, 0.5);

                    ui.separator();
                    ui.heading("Отступы (egui::Spacing)");
                    changed |= drag2(ui, "Между виджетами [x, y]", &mut theme.item_spacing, 0.0, 64.0, 0.5);
                    changed |= drag2(ui, "Окно [x, y]", &mut theme.window_margin, 0.0, 64.0, 0.5);
                    changed |= drag2(ui, "Меню [x, y]", &mut theme.menu_margin, 0.0, 64.0, 0.5);
                    changed |= drag(ui, "Иерархия (indent)", &mut theme.indent, 0.0, 64.0, 0.5);
                    changed |= drag(ui, "Выпадающий список", &mut theme.combo_height, 40.0, 600.0, 1.0);
                    changed |= drag(ui, "Ползунок", &mut theme.slider_width, 40.0, 600.0, 1.0);
                    changed |= drag(ui, "Маркер размера", &mut theme.resize_corner_size, 0.0, 64.0, 0.5);

                    ui.separator();
                    ui.heading("Скругление");
                    changed |= drag(ui, "Углы окон", &mut theme.window_corner_radius, 0.0, 64.0, 0.5);
                    changed |= drag(ui, "Углы виджетов", &mut theme.widget_rounding, 0.0, 32.0, 0.5);

                    ui.separator();
                    ui.heading("Текстовые стили (px)");
                    changed |= drag(ui, "Основной", &mut theme.text_size_body, 6.0, 72.0, 0.5);
                    changed |= drag(ui, "Кнопки", &mut theme.text_size_button, 6.0, 72.0, 0.5);
                    changed |= drag(ui, "Маленький", &mut theme.text_size_small, 6.0, 72.0, 0.5);
                    changed |= drag(ui, "Заголовки", &mut theme.text_size_heading, 8.0, 120.0, 0.5);
                    changed |= drag(ui, "Моноширина", &mut theme.text_size_monospace, 6.0, 72.0, 0.5);

                    ui.separator();
                    ui.heading("Тень и свечение");
                    changed |= bool_(ui, "Тень", &mut theme.shadow_enabled);
                    if theme.shadow_enabled {
                        changed |= drag2(ui, "Смещение [x, y]", &mut theme.shadow_offset, -64.0, 64.0, 0.5);
                        changed |= drag(ui, "Размытие", &mut theme.shadow_blur, 0.0, 64.0, 0.5);
                        changed |= color(ui, "Цвет тени", &mut theme.shadow_color);
                    }
                    changed |= bool_(ui, "Свечение", &mut theme.glow_enabled);
                    if theme.glow_enabled {
                        changed |= drag(ui, "Размер", &mut theme.glow_size, 0.0, 64.0, 0.5);
                        changed |= color(ui, "Цвет свечения", &mut theme.glow_color);
                    }

                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Сброс к значениям по умолчанию").clicked() {
                            *theme = Theme::default();
                            changed = true;
                        }
                        if ui.button("Применить как значение по умолчанию").clicked() {
                            changed = true;
                            save_default = true;
                        }
                        if ui.button("Закрыть").clicked() {
                            close_requested = true;
                        }
                    });
                });
            });

        if save_default {
            app.add_event_log("info", "theme", "Тема сохранена как значение по умолчанию", None);
        }

        if close_requested {
            open = false;
        }
        app.show_settings = open;
    }

    changed
}

/// Ячейка цвета с меткой. При нажатии на ячейку открывается всплывающая палитра.
fn color(ui: &mut egui::Ui, label: &str, value: &mut egui::Color32) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        let swatch = ui.add(
            egui::Button::new("")
                .fill(*value)
                .min_size(egui::vec2(16.0, 16.0)),
        );
        egui::Popup::from_toggle_button_response(&swatch).show(|popup_ui| {
            popup_ui.label(label);
            if color_picker_color32(popup_ui, value, Alpha::Opaque) {
                changed = true;
            }
        });
    });
    changed
}

/// Одно f32 значение.
fn drag(ui: &mut egui::Ui, label: &str, value: &mut f32, min: f32, max: f32, speed: f32) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        let dv = egui::DragValue::new(value).speed(speed).range(min..=max);
        if ui.add(dv).changed() {
            changed = true;
        }
    });
    changed
}

/// Два f32 значения ([x, y]).
fn drag2(ui: &mut egui::Ui, label: &str, value: &mut [f32; 2], min: f32, max: f32, speed: f32) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        let cx = ui.add(egui::DragValue::new(&mut value[0]).speed(speed).range(min..=max)).changed();
        let cy = ui.add(egui::DragValue::new(&mut value[1]).speed(speed).range(min..=max)).changed();
        if cx || cy {
            changed = true;
        }
    });
    changed
}

/// Флажок bool.
fn bool_(ui: &mut egui::Ui, label: &str, value: &mut bool) -> bool {
    ui.checkbox(value, label).changed()
}

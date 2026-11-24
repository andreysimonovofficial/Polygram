//! Модальное диалоговое окно добавления показателя.
//! Соответствует спецификации оформления: «Модальные окна - Оформление.md» (в т.ч. правки v15).
//! Универсальная оболочка (оверлей, рамка, шапка, подвал, состояния)
//! вынесена в `polygram_canvas::modal`; здесь осталось только специфическое
//! тело окна добавления показателя с жёстким вертикальным центрированием
//! содержимого внутри рамки тела и составным блоком «значение + синхронизация».

use crate::app::PolygramApp;
use polygram_canvas::modal::{
    footer, header, modal_frame, paint_overlay,
    MODAL_BG, MODAL_BORDER, MODAL_INPUT_BG, MODAL_INPUT_HOVER_BG,
    MODAL_INPUT_HOVER_BORDER, MODAL_TEXT, MODAL_TEXT_MUTED, MODAL_TYPE_ACTIVE_BG,
    MODAL_TYPE_HOVER_BG,
};
use polygram_canvas::paint_lucide;
use chrono::{Datelike, NaiveDate, NaiveTime, Utc};
use egui::{
    Align, Color32, Context, CornerRadius, Frame, Layout, Margin, PointerButton, Rect,
    Sense, Stroke, StrokeKind, Ui, Vec2, pos2, Window,
};
use egui_lucide::Lucide;
use polygram_core::{MetricKind, MetricValue, runtime::Published};

/// Фон внутренней рамки тела окна: #222222.
const BG_VALUE_ROW: Color32 = Color32::from_rgb(0x22, 0x22, 0x22);
/// Фон кнопки синхронизации календаря: #2B2B2B.
const BG_SYNC_BASE: Color32 = Color32::from_rgb(0x2B, 0x2B, 0x2B);
/// Фон кнопки синхронизации при наведении: #323232.
const BG_SYNC_HOVER: Color32 = Color32::from_rgb(0x32, 0x32, 0x32);
/// Фон кнопки синхронизации при нажатии: #383838.
const BG_SYNC_ACTIVE: Color32 = Color32::from_rgb(0x38, 0x38, 0x38);

/// Иконка Lucide для данного типа показателя.
fn metric_kind_icon(kind: MetricKind) -> Lucide {
    match kind {
        MetricKind::Number => Lucide::Hash,
        MetricKind::Time => Lucide::Clock,
        MetricKind::Date => Lucide::Calendar,
        MetricKind::Deadline => Lucide::CalendarClock,
        MetricKind::Percentage => Lucide::Percent,
        MetricKind::Binary => Lucide::ToggleRight,
        MetricKind::Duration => Lucide::Clock,
        MetricKind::Money => Lucide::DollarSign,
        MetricKind::Text => Lucide::FileText,
        MetricKind::Formula => Lucide::Calculator,
        MetricKind::Timestamp => Lucide::CalendarClock,
        _ => Lucide::Hash,
    }
}

/// Следующий тип показателя при клике на иконку типа.
fn metric_kind_next(kind: MetricKind) -> MetricKind {
    match kind {
        MetricKind::Number => MetricKind::Time,
        MetricKind::Time => MetricKind::Date,
        MetricKind::Date => MetricKind::Deadline,
        MetricKind::Deadline => MetricKind::Percentage,
        MetricKind::Percentage => MetricKind::Binary,
        MetricKind::Binary => MetricKind::Duration,
        MetricKind::Duration => MetricKind::Money,
        MetricKind::Money => MetricKind::Text,
        MetricKind::Text => MetricKind::Formula,
        MetricKind::Formula => MetricKind::Timestamp,
        MetricKind::Timestamp => MetricKind::Number,
        _ => MetricKind::Number,
    }
}

/// Является ли тип показателя датовым (Date / Deadline).
fn is_date_kind(kind: MetricKind) -> bool {
    matches!(kind, MetricKind::Date | MetricKind::Deadline)
}

/// Модальное диалоговое окно добавления показателя.
pub fn handle_add_metric_dialog(
    app: &mut PolygramApp,
    ctx: &Context,
    published: &Published,
) {
    let Some((object_id, dir)) = &app.dialogs.add_metric_target.clone() else {
        return;
    };
    let object_id = object_id.clone();
    let dir = *dir;

    let label = match dir {
        polygram_core::types::Direction::Request => "ЗАПРОС",
        polygram_core::types::Direction::Result => "РЕЗУЛЬТАТ",
    };

    let mut done = false;
    let mut cancelled = false;

    let existing_md = published
        .metric_defs
        .iter()
        .find(|md| md.name == app.dialogs.add_metric_name.trim())
        .cloned();

    // Затемнённый фон позади модального окна (50%).
    paint_overlay(ctx);

    // Центрирование модального окна по экрану (размер 540×197 px).
    let screen_rect = ctx.viewport_rect();
    let center_pos = egui::pos2(
        screen_rect.center().x - 270.0,
        screen_rect.center().y - 98.5,
    );

    Window::new("")
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .fixed_size([540.0, 197.0])
        .fixed_pos(center_pos)
        .frame(modal_frame())
        .show(ctx, |ui| {
            // ---- Шапка диалога (отступы 20/20/20/12, фон #1E1E1E) ----
            let closed = header(ui, &app.dialogs.add_metric_process_name, label);
            if closed {
                cancelled = true;
            }

            // ---- Тело диалога (отступы секции: сверху 4, слева 20, справа 20, снизу 16) ----
            Frame::new()
                .fill(MODAL_BG)
                .inner_margin(Margin {
                    left: 20,
                    top: 4,
                    right: 20,
                    bottom: 16,
                })
                .show(ui, |ui| {
                    // Внутренняя рамка тела: 498×52, bg #222222, рамка 1 px #373839, radius 4.
                    let frame_size = Vec2::new(498.0, 52.0);
                    let (frame_rect, _) =
                        ui.allocate_exact_size(frame_size, Sense::hover());
                    ui.painter()
                        .rect_filled(frame_rect, CornerRadius::same(4), BG_VALUE_ROW);
                    ui.painter().rect_stroke(
                        frame_rect,
                        CornerRadius::same(4),
                        Stroke::new(1.0, MODAL_BORDER),
                        StrokeKind::Middle,
                    );

                    // Внутренняя область с отступом 8 px; все элементы строго по вертикальному центру.
                    const PAD: f32 = 8.0;
                    let work = Rect::from_min_max(
                        pos2(frame_rect.min.x + PAD, frame_rect.min.y + PAD),
                        pos2(frame_rect.max.x - PAD, frame_rect.max.y - PAD),
                    );
                    // work = 482×36; элементы высотой 34 -> центр, отступ по 1 px от границ work.
                    const ELEM_H: f32 = 34.0;
                    let elem_y = work.min.y + (work.height() - ELEM_H) / 2.0;

                    let mut x = work.min.x;

                    // ---- 1. Кнопка смены типа показателя (34×34, radius 4) ----
                    {
                        let type_rect =
                            Rect::from_min_size(pos2(x, elem_y), Vec2::splat(34.0));
                        let type_response = ui.interact(
                            type_rect,
                            ui.auto_id_with("type_button"),
                            Sense::click(),
                        );
                        let hovered = type_response.hovered();
                        let pressed = hovered
                            && ui.ctx().input(|i| {
                                i.pointer.button_down(PointerButton::Primary)
                            });
                        let fill = if pressed {
                            MODAL_TYPE_ACTIVE_BG
                        } else if hovered {
                            MODAL_TYPE_HOVER_BG
                        } else {
                            MODAL_INPUT_BG
                        };
                        ui.painter()
                            .rect_filled(type_rect, CornerRadius::same(4), fill);
                        ui.painter().rect_stroke(
                            type_rect,
                            CornerRadius::same(4),
                            Stroke::new(1.0, MODAL_BORDER),
                            StrokeKind::Middle,
                        );
                        let icon_rect =
                            Rect::from_center_size(type_rect.center(), Vec2::splat(16.0));
                        paint_lucide(
                            ui,
                            icon_rect,
                            metric_kind_icon(app.dialogs.add_metric_kind),
                            MODAL_TEXT,
                        );
                        if type_response.clicked() {
                            let kind =
                                metric_kind_next(app.dialogs.add_metric_kind);
                            app.dialogs.add_metric_kind = kind;
                            app.dialogs.add_metric_value.clear();
                            app.dialogs.add_metric_date = None;
                            app.dialogs.add_metric_date_popup_open = false;
                            app.dialogs.add_metric_time = None;
                        }
                        x += 34.0 + 8.0;
                    }

                    // ---- 2. Имя показателя (растягивается, 34 px, radius 2, pad 10/8/7.5/7.5) ----
                    {
                        let name_w = work.width() - 34.0 - 130.0 - 88.0 - 3.0 * 8.0;
                        let name_rect =
                            Rect::from_min_size(pos2(x, elem_y), Vec2::new(name_w, ELEM_H));
                        text_field(
                            ui,
                            name_rect,
                            &mut app.dialogs.add_metric_name,
                            "Имя Показателя",
                            false,
                            10.0,
                            10.0,
                            7.5,
                            7.5,
                        );
                        x += name_w + 8.0;
                    }

                    // ---- 3. Составной блок значение + синхронизация (130×34, radius 2) ----
                    {
                        let comp_rect =
                            Rect::from_min_size(pos2(x, elem_y), Vec2::new(130.0, ELEM_H));
                        // Единая рамка контейнера: bg #252525, рамка 1 px #373839, radius 2.
                        ui.painter()
                            .rect_filled(comp_rect, CornerRadius::same(2), MODAL_INPUT_BG);
                        ui.painter().rect_stroke(
                            comp_rect,
                            CornerRadius::same(2),
                            Stroke::new(1.0, MODAL_BORDER),
                            StrokeKind::Middle,
                        );
                        // Внутренняя область (за вычетом рамки 1 px): 128×32.
                        let inner = Rect::from_min_max(
                            pos2(comp_rect.min.x + 1.0, comp_rect.min.y + 1.0),
                            pos2(comp_rect.max.x - 1.0, comp_rect.max.y - 1.0),
                        );

                        const VALUE_W: f32 = 96.0;

                        if is_date_kind(app.dialogs.add_metric_kind) {
                            // Кнопка даты/времени 96×32 внутри зоны значения.
                            let date_rect = Rect::from_min_size(
                                pos2(inner.min.x, inner.min.y),
                                Vec2::new(VALUE_W, inner.height()),
                            );
                            let date_response = ui.interact(
                                date_rect,
                                ui.auto_id_with("date_button"),
                                Sense::click(),
                            );
                            let hovered = date_response.hovered();
                            let pressed = hovered
                                && ui.ctx().input(|i| {
                                    i.pointer.button_down(PointerButton::Primary)
                                });
                            let fill = if pressed || hovered {
                                MODAL_INPUT_HOVER_BG
                            } else {
                                MODAL_INPUT_BG
                            };
                            let stroke = if hovered || pressed {
                                MODAL_INPUT_HOVER_BORDER
                            } else {
                                MODAL_BORDER
                            };
                            ui.painter()
                                .rect_filled(date_rect, CornerRadius::same(2), fill);
                            ui.painter().rect_stroke(
                                date_rect,
                                CornerRadius::same(2),
                                Stroke::new(1.0, stroke),
                                StrokeKind::Middle,
                            );
                            if date_response.clicked() {
                                app.dialogs.add_metric_date_popup_open = true;
                            }
                            if app.dialogs.add_metric_date_popup_open {
                                let mut picked = false;
                                egui::Window::new("Выберите дату")
                                    .collapsible(false)
                                    .resizable(false)
                                    .show(ctx, |ui| {
                                        if render_date_time_popup(
                                            ui,
                                            &mut app.dialogs.add_metric_date,
                                            &mut app.dialogs.add_metric_time,
                                        ) {
                                            picked = true;
                                        }
                                    });
                                if picked {
                                    app.dialogs.add_metric_date_popup_open = false;
                                }
                            }
                            app.dialogs.add_metric_value = match (
                                app.dialogs.add_metric_date,
                                app.dialogs.add_metric_time,
                            ) {
                                (Some(d), Some(_t)) => {
                                    d.format("%d.%m.%Y %H:%M").to_string()
                                }
                                (Some(d), None) => d.format("%d.%m.%Y").to_string(),
                                _ => String::new(),
                            };
                            let date_label = match (
                                app.dialogs.add_metric_date,
                                app.dialogs.add_metric_time,
                            ) {
                                (Some(d), Some(t)) => {
                                    d.format("%d.%m.%Y").to_string()
                                        + " "
                                        + &t.format("%H:%M").to_string()
                                }
                                (Some(d), None) => d.format("%d.%m.%Y").to_string(),
                                (None, Some(t)) => {
                                    "дата ".to_owned() + &t.format("%H:%M").to_string()
                                }
                                _ => "дата".to_string(),
                            };
                            paint_right_aligned(
                                ui,
                                date_rect,
                                &date_label,
                                8.0,
                            );
                        } else {
                            // Поле ввода значения: правое выравнивание, pad 8/8/7.5/7.5.
                            let value_text_rect = Rect::from_min_max(
                                pos2(inner.min.x + 8.0, inner.min.y + 7.5),
                                pos2(inner.min.x + VALUE_W - 8.0, inner.max.y - 7.5),
                            );
                            let mut child = ui.new_child(
                                egui::UiBuilder::new()
                                    .max_rect(value_text_rect)
                                    .layout(Layout::right_to_left(Align::Center)),
                            );
                            child.add(
                                egui::TextEdit::singleline(
                                    &mut app.dialogs.add_metric_value,
                                )
                                .frame(
                                    Frame::new()
                                        .fill(Color32::TRANSPARENT)
                                        .stroke(Stroke::new(
                                            0.0,
                                            Color32::TRANSPARENT,
                                        )),
                                )
                                .text_color(MODAL_TEXT)
                                .hint_text("Значение"),
                            );
                        }

                        // Разделительная линия 1 px #373839 на границе между значением и синхронизацией.
                        let divider_x = inner.min.x + VALUE_W;
                        ui.painter().line_segment(
                            [
                                pos2(divider_x, inner.min.y),
                                pos2(divider_x, inner.max.y),
                            ],
                            Stroke::new(1.0, MODAL_BORDER),
                        );

                        // Кнопка синхронизации 32×32, левая граница — разделительная линия.
                        const SYNC_W: f32 = 32.0;
                        let sync_rect = Rect::from_min_size(
                            pos2(inner.min.x + VALUE_W, inner.min.y),
                            Vec2::new(SYNC_W, inner.height()),
                        );
                        let sync_response = ui.interact(
                            sync_rect,
                            ui.auto_id_with("sync_button"),
                            Sense::click(),
                        );
                        let hovered = sync_response.hovered();
                        let pressed = hovered
                            && ui.ctx().input(|i| {
                                i.pointer.button_down(PointerButton::Primary)
                            });
                        let fill = if pressed {
                            BG_SYNC_ACTIVE
                        } else if hovered {
                            BG_SYNC_HOVER
                        } else {
                            BG_SYNC_BASE
                        };
                        // Заливка кнопки синхронизации (кроме левой 1 px границы-разделителя).
                        let sync_fill_rect = Rect::from_min_size(
                            pos2(sync_rect.min.x + 1.0, sync_rect.min.y),
                            Vec2::new(SYNC_W - 1.0, sync_rect.height()),
                        );
                        ui.painter().rect_filled(sync_fill_rect, CornerRadius::same(0), fill);
                        let icon_rect =
                            Rect::from_center_size(sync_rect.center(), Vec2::splat(15.0));
                        paint_lucide(ui, icon_rect, Lucide::CalendarSync, MODAL_TEXT_MUTED);
                        // Кнопка является визуальной заглушкой: при нажатии ничего не происходит.
                        let _ = sync_response;

                        x += 130.0 + 8.0;
                    }

                    // ---- 4. Единица измерения (88×34, правое выравнивание, radius 2, pad 10/8) ----
                    {
                        let unit_rect =
                            Rect::from_min_size(pos2(x, elem_y), Vec2::new(88.0, ELEM_H));
                        text_field(
                            ui,
                            unit_rect,
                            &mut app.dialogs.add_metric_unit,
                            "Ед.",
                            true,
                            10.0,
                            10.0,
                            8.0,
                            8.0,
                        );
                    }
                });

            ui.add_space(12.0);

            // ---- Нижняя панель действий (высота 52, отступы 20/20/20, вправо, по центру вертикали) ----
            let confirmed = footer(ui);
            if confirmed {
                done = true;
            }
        });

    if cancelled {
        app.dialogs.reset_add_metric();
        return;
    }

    if done {
        let name = app.dialogs.add_metric_name.trim().to_owned();
        if name.is_empty() {
            app.dialogs.reset_add_metric();
            return;
        }

        let value = parse_metric_value(&app.dialogs.add_metric_value, app.dialogs.add_metric_kind);
        let object_id_for_cmd = object_id.clone();
        let name_for_cmd = name.clone();

        let kind = app.dialogs.add_metric_kind;
        let unit = app.dialogs.add_metric_unit.trim().to_owned();
        let (entry_value, entry_required) = match dir {
            polygram_core::types::Direction::Request => (MetricValue::Null, value.clone()),
            polygram_core::types::Direction::Result => (value.clone(), MetricValue::Null),
        };

        let reuse_existing = existing_md.as_ref().map(|md| kind == md.kind).unwrap_or(false);

        if reuse_existing {
            app.apply_cmd(polygram_core::Command::AddMetricEntry {
                object: object_id_for_cmd.clone(),
                dir,
                id: Some(name_for_cmd.clone()),
                metric_def: existing_md.as_ref().unwrap().name.clone(),
                name: name_for_cmd.clone(),
                value: entry_value,
                required: entry_required,
                max: None,
                fact: MetricValue::Null,
            });
        } else {
            app.apply_batch(vec![
                polygram_core::Command::AddMetricDef {
                    id: Some(name_for_cmd.clone()),
                    name: name_for_cmd.clone(),
                    kind,
                    unit,
                    api: None,
                },
                polygram_core::Command::AddMetricEntry {
                    object: object_id_for_cmd.clone(),
                    dir,
                    id: Some(name_for_cmd.clone()),
                    metric_def: name_for_cmd.clone(),
                    name: name_for_cmd.clone(),
                    value: entry_value,
                    required: entry_required,
                    max: None,
                    fact: MetricValue::Null,
                },
            ]);
        }

        app.dialogs.reset_add_metric();
    }
}

/// Текстовое поле модального окна в заданном прямоугольнике.
/// Рисует рамку (с состоянием наведения) и размещает `TextEdit` со выравниванием
/// текста по вертикальному центру; `right_align` — текст прижат к правому краю.
fn text_field(
    ui: &mut Ui,
    rect: Rect,
    text: &mut String,
    hint: &str,
    right_align: bool,
    pad_l: f32,
    pad_r: f32,
    pad_t: f32,
    pad_b: f32,
) {
    let hovered = ui
        .ctx()
        .input(|i| i.pointer.hover_pos())
        .map_or(false, |p| rect.contains(p));
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
    ui.painter().rect_filled(rect, CornerRadius::same(2), fill);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(2),
        Stroke::new(1.0, stroke),
        StrokeKind::Middle,
    );

    let inner = Rect::from_min_max(
        pos2(rect.min.x + pad_l, rect.min.y + pad_t),
        pos2(rect.max.x - pad_r, rect.max.y - pad_b),
    );
    let layout = if right_align {
        Layout::right_to_left(Align::Center)
    } else {
        Layout::left_to_right(Align::Center)
    };
    let mut child = ui.new_child(
        egui::UiBuilder::new().max_rect(inner).layout(layout),
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
}

/// Окрасить текст строкой, прижатый к правому краю прямоугольника, по вертикальному центру.
fn paint_right_aligned(ui: &Ui, rect: Rect, text: &str, pad_right: f32) {
    let font_id = egui::FontId::new(14.0, egui::FontFamily::Proportional);
    let galley = ui
        .fonts_mut(|f| f.layout_no_wrap(text.to_owned(), font_id, MODAL_TEXT));
    let x = rect.max.x - pad_right - galley.size().x;
    let y = rect.center().y - galley.size().y / 2.0;
    ui.painter()
        .galley(pos2(x, y), galley, MODAL_TEXT);
}

/// Преобразовать строку из текстового поля в MetricValue согласно типу показателя.
fn parse_metric_value(s: &str, kind: MetricKind) -> MetricValue {
    let trimmed = s.trim();
    match kind {
        MetricKind::Number => trimmed
            .parse::<f64>()
            .map(MetricValue::Number)
            .unwrap_or(MetricValue::Null),
        MetricKind::Money => trimmed
            .parse::<f64>()
            .map(|v| MetricValue::Money(v as i64))
            .unwrap_or(MetricValue::Null),
        MetricKind::Percentage => trimmed
            .parse::<f64>()
            .map(MetricValue::Percentage)
            .unwrap_or(MetricValue::Null),
        MetricKind::Duration => trimmed
            .parse::<f64>()
            .map(|v| MetricValue::Duration(v as i64))
            .unwrap_or(MetricValue::Null),
        MetricKind::Date | MetricKind::Deadline => {
            if let Ok(dt) = NaiveDate::parse_from_str(trimmed, "%d.%m.%Y %H:%M")
                .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc())
                .or_else(|_| {
                    NaiveDate::parse_from_str(trimmed, "%d.%m.%Y").map(|d| {
                        d.and_hms_opt(0, 0, 0).unwrap().and_utc()
                    })
                })
                .or_else(|_| {
                    chrono::DateTime::parse_from_rfc3339(trimmed)
                        .map(|dt| dt.with_timezone(&Utc))
                })
            {
                match kind {
                    MetricKind::Deadline => MetricValue::Deadline(dt),
                    _ => MetricValue::Date(dt),
                }
            } else {
                MetricValue::Null
            }
        }
        MetricKind::Binary => match trimmed.to_lowercase().as_str() {
            "true" | "1" | "да" | "yes" => MetricValue::Binary(true),
            "false" | "0" | "нет" | "no" => MetricValue::Binary(false),
            _ => MetricValue::Null,
        },
        MetricKind::Text => MetricValue::Text(trimmed.to_owned()),
        _ => MetricValue::Null,
    }
}

/// Рисовать popup-календарь с выбором времени для типа Date / Deadline.
fn render_date_time_popup(
    ui: &mut egui::Ui,
    selected_date: &mut Option<NaiveDate>,
    selected_time: &mut Option<NaiveTime>,
) -> bool {
    let now = chrono::Local::now();
    let show_date = selected_date.unwrap_or(now.date_naive());
    let _show_time = selected_time.unwrap_or(now.time());

    let month_names = [
        "Янв", "Фев", "Мар", "Апр", "Май", "Июн",
        "Июл", "Авг", "Сен", "Окт", "Ноя", "Дек",
    ];

    let prev_month = |d: NaiveDate| -> NaiveDate {
        let (y, m) = if d.month() == 1 {
            (d.year() - 1, 12)
        } else {
            (d.year(), d.month() - 1)
        };
        d.with_day(1)
            .unwrap()
            .with_year(y)
            .unwrap()
            .with_month(m as u32)
            .unwrap()
    };
    let next_month = |d: NaiveDate| -> NaiveDate {
        let (y, m) = if d.month() == 12 {
            (d.year() + 1, 1)
        } else {
            (d.year(), d.month() + 1)
        };
        d.with_day(1)
            .unwrap()
            .with_year(y)
            .unwrap()
            .with_month(m as u32)
            .unwrap()
    };

    let mut picked = false;

    egui::Grid::new("date_time_calendar").num_columns(7).show(ui, |ui| {
        ui.horizontal(|ui| {
            if ui.button("‹").clicked() {
                *selected_date = Some(prev_month(show_date));
            }
            ui.label(format!(
                "{} {}",
                month_names[(show_date.month() - 1) as usize],
                show_date.year()
            ));
            if ui.button("›").clicked() {
                *selected_date = Some(next_month(show_date));
            }
        });
        ui.end_row();

        for wd in ["Пн", "Вт", "Ср", "Чт", "Пт", "Сб", "Вс"] {
            ui.label(wd);
        }
        ui.end_row();

        let first = show_date.with_day(1).unwrap();
        let next = next_month(show_date);
        let days_in_month = (next - first).num_days() as u32;
        let start_offset = first.weekday().number_from_monday() as usize - 1;
        let total_cells = start_offset + days_in_month as usize;
        let rows = total_cells.div_ceil(7);

        for row in 0..rows {
            for col in 0..7 {
                let idx = row * 7 + col;
                if idx < start_offset || idx >= total_cells {
                    ui.label("");
                } else {
                    let day = (idx - start_offset + 1) as u32;
                    let date = show_date.with_day(day).unwrap();
                    let is_sel = selected_date.map_or(false, |s| s == date);
                    let mut button = egui::Button::new(format!("{}", day));
                    if is_sel {
                        button = button.frame(true);
                    }
                    if ui.add(button).clicked() {
                        *selected_date = Some(date);
                        picked = true;
                    }
                }
            }
            ui.end_row();
        }
    });

    ui.separator();
    ui.horizontal(|ui| {
        ui.label("Время:");
        let mut time_str = selected_time
            .map(|t| t.format("%H:%M").to_string())
            .unwrap_or_else(|| "00:00".to_owned());
        let time_response = ui.add(
            egui::TextEdit::singleline(&mut time_str)
                .desired_width(50.0)
                .hint_text("чч:мм"),
        );
        if time_response.changed() {
            if let Ok(t) = NaiveTime::parse_from_str(time_str.trim(), "%H:%M") {
                *selected_time = Some(t);
            } else if let Ok(t) = NaiveTime::parse_from_str(time_str.trim(), "%H:%M:%S") {
                *selected_time = Some(t);
            } else if time_str.trim().is_empty() {
                *selected_time = None;
            }
        }
        if ui.button("сегодня").clicked() {
            *selected_date = Some(now.date_naive());
            *selected_time = Some(now.time());
            picked = true;
        }
    });

    picked
}

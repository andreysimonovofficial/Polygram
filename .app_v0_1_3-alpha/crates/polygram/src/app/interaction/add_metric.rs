//! Модальное диалоговое окно добавления показателя.
//! Соответствует спецификации оформления: «Модальные окна - Оформление.md».
//! Размер диалога 540×198 px, заголовок с именем процесса,
//! переключение типа показателя по иконке (# для Number, Часы для Time и т.д.),
//! кнопка calendar-sync в виде заглушки (no-op).

use crate::app::PolygramApp;
use crate::app::icons::lucide_button;
use chrono::{Datelike, NaiveDate, NaiveTime, Utc};
use egui::{
    Align, Color32, Context, CornerRadius, Frame, Layout, Margin, Stroke, Vec2, Window,
};
use egui_lucide::Lucide;
use polygram_core::{MetricKind, MetricValue, runtime::Published};

/// Цвет фона диалога: #1E1E1E.
const BG_DIALOG: Color32 = Color32::from_rgb(0x1E, 0x1E, 0x1E);
/// Цвет рамки: #373839.
const BG_BORDER: Color32 = Color32::from_rgb(0x37, 0x38, 0x39);
/// Цвет фона полей ввода: #252525.
const BG_INPUT: Color32 = Color32::from_rgb(0x25, 0x25, 0x25);
/// Цвет фона строки значения: #222222.
const BG_VALUE_ROW: Color32 = Color32::from_rgb(0x22, 0x22, 0x22);
/// Цвет фона кнопки calendar-sync: #2B2B2B.
const BG_CALENDAR_STUB: Color32 = Color32::from_rgb(0x2B, 0x2B, 0x2B);
/// Цвет белого: #FFFFFF.
const COLOR_WHITE: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
/// Цвет серого текста: #A1A1AA.
const COLOR_GRAY: Color32 = Color32::from_rgb(0xA1, 0xA1, 0xAA);

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
    let value_label = match dir {
        polygram_core::types::Direction::Request => "Необходимое (required):",
        polygram_core::types::Direction::Result => "Значение (value):",
    };

    let mut done = false;
    let mut cancelled = false;

    let existing_md = published
        .metric_defs
        .iter()
        .find(|md| md.name == app.dialogs.add_metric_name.trim())
        .cloned();

    Window::new("")
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .fixed_size([540.0, 198.0])
        .frame(Frame::new().inner_margin(Margin::same(0)))
        .show(ctx, |ui| {
            // Фон диалога и рамка (540×198, bg #1E1E1E, border 1px #373839, radius 8)
            let dialog_rect = ui.available_rect_before_wrap();
            {
                let p = ui.painter();
                p.rect_filled(dialog_rect, 8.0, BG_DIALOG);
                p.rect_stroke(
                    dialog_rect,
                    8.0,
                    Stroke::new(1.0, BG_BORDER),
                    egui::StrokeKind::Middle,
                );
            }

            // ---- Шапка диалога (Background section) ----
            // flex-direction: row, justify-content: space-between, align-items: flex-start
            // padding: 20px 20px 12px, gap: 277.84px, размер: 538×72, bg: #1E1E1E
            let header_rect = egui::Rect::from_min_size(
                dialog_rect.min + Vec2::new(1.0, 1.0),
                Vec2::new(538.0, 72.0),
            );
            {
                let p = ui.painter();
                p.rect_filled(header_rect, 0.0, BG_DIALOG);
            }
            ui.allocate_rect(header_rect, egui::Sense::hover());

            ui.horizontal(|ui| {
                ui.set_min_width(538.0);
                ui.spacing_mut().item_spacing = Vec2::new(277.84, 0.0);

                // ---- Контейнер имени процесса (Column, gap 4px, 192.16×40) ----
                ui.vertical(|ui| {
                    ui.set_min_width(192.16);
                    ui.spacing_mut().item_spacing = Vec2::new(0.0, 4.0);

                    // Имя процесса (Inter 600, 16px, #FFFFFF)
                    ui.label(
                        egui::RichText::new(&app.dialogs.add_metric_process_name)
                            .strong()
                            .size(16.0)
                            .color(COLOR_WHITE),
                    );

                    // Строка типа показателя (Row, center, gap 6px, 192.16×14)
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(6.0, 0.0);

                        // Точка-индикатор (6×6, #A1A1AA, border-radius 3px)
                        let dot_center = ui.cursor().center();
                        {
                            let p = ui.painter();
                            p.circle_filled(dot_center, 3.0, COLOR_GRAY);
                        }
                        ui.add_space(6.0);

                        // Контейнер метки типа (Column, 49.92×14)
                        ui.vertical(|ui| {
                            ui.set_min_width(49.92);
                            ui.label(
                                egui::RichText::new("ТИП")
                                    .strong()
                                    .small()
                                    .extra_letter_spacing(0.44)
                                    .text_style(egui::TextStyle::Small)
                                    .color(COLOR_GRAY),
                            );
                        });
                    });
                });

                // ---- Кнопка закрытия (28×28, border-radius 4px) ----
                ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                    ui.set_min_width(28.0);
                    let (rect, response) = ui.allocate_exact_size(
                        Vec2::splat(28.0),
                        egui::Sense::click(),
                    );
                    if response.hovered() {
                        let p = ui.painter();
                        p.rect_filled(rect, 0.0, BG_INPUT);
                    }
                    lucide_button(ui, Lucide::X, 16.0, COLOR_GRAY);
                });
            });

            ui.add_space(4.0);

            // ---- Основное содержимое (Container, column, padding 4px 20px 16px, 538×72) ----
            let content_rect = egui::Rect::from_min_size(
                dialog_rect.min + Vec2::new(20.0, 76.0),
                Vec2::new(498.0, 52.0),
            );
            {
                let p = ui.painter();
                p.rect_filled(content_rect, 4.0, BG_VALUE_ROW);
                p.rect_stroke(
                    content_rect,
                    4.0,
                    Stroke::new(1.0, BG_BORDER),
                    egui::StrokeKind::Middle,
                );
            }
            ui.allocate_rect(content_rect, egui::Sense::hover());

            ui.horizontal(|ui| {
                ui.set_min_width(498.0);
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);

                // ---- Кнопка иконки типа (Button, 34×34) ----
                // bg: #252525, border: 1px #373839, border-radius: 4px, SVG 16×16
                let icon_rect = egui::Rect::from_min_size(
                    ui.cursor().min,
                    Vec2::splat(34.0),
                );
                {
                    let p = ui.painter();
                    p.rect_filled(icon_rect, 4.0, BG_INPUT);
                    p.rect_stroke(
                        icon_rect,
                        4.0,
                        Stroke::new(1.0, BG_BORDER),
                        egui::StrokeKind::Middle,
                    );
                }

                let icon_response = lucide_button(
                    ui,
                    metric_kind_icon(app.dialogs.add_metric_kind),
                    16.0,
                    COLOR_WHITE,
                );
                if icon_response.clicked() {
                    app.dialogs.add_metric_kind =
                        metric_kind_next(app.dialogs.add_metric_kind);
                    app.dialogs.add_metric_value.clear();
                    app.dialogs.add_metric_date = None;
                    app.dialogs.add_metric_date_popup_open = false;
                    app.dialogs.add_metric_time = None;
                }

                ui.add_space(8.0);

                // ---- Поле ввода значения (Input) ----
                // padding: 7.5px 10px, размер: 204×34, bg: #252525, border: 1px #373839, border-radius: 2px
                let input_rect = egui::Rect::from_min_size(
                    ui.cursor().min,
                    Vec2::new(204.0, 34.0),
                );
                {
                    let p = ui.painter();
                    p.rect_filled(input_rect, 2.0, BG_INPUT);
                    p.rect_stroke(
                        input_rect,
                        2.0,
                        Stroke::new(1.0, BG_BORDER),
                        egui::StrokeKind::Middle,
                    );
                }
                ui.allocate_rect(input_rect, egui::Sense::hover());

                ui.vertical_centered(|ui| {
                    ui.set_min_size(Vec2::new(204.0, 34.0));
                    let is_date_kind = matches!(
                        app.dialogs.add_metric_kind,
                        MetricKind::Date | MetricKind::Deadline
                    );
                    if is_date_kind {
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
                        if ui.button(date_label).clicked() {
                            app.dialogs.add_metric_date_popup_open = true;
                        }
                        if app.dialogs.add_metric_date_popup_open {
                            let picked = render_date_time_popup(
                                ui,
                                &mut app.dialogs.add_metric_date,
                                &mut app.dialogs.add_metric_time,
                            );
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
                            (Some(d), None) => {
                                d.format("%d.%m.%Y").to_string()
                            }
                            _ => String::new(),
                        };
                    } else {
                        ui.text_edit_singleline(&mut app.dialogs.add_metric_value);
                    }
                });

                ui.add_space(8.0);

                // ---- Единица измерения (Input, column) ----
                // padding: 7.5px 8px, размер: 96×32, bg: #252525, border: 1px #373839, border-radius: 2px
                let unit_rect = egui::Rect::from_min_size(
                    ui.cursor().min,
                    Vec2::new(96.0, 32.0),
                );
                {
                    let p = ui.painter();
                    p.rect_filled(unit_rect, 2.0, BG_INPUT);
                    p.rect_stroke(
                        unit_rect,
                        2.0,
                        Stroke::new(1.0, BG_BORDER),
                        egui::StrokeKind::Middle,
                    );
                }
                ui.allocate_rect(unit_rect, egui::Sense::hover());

                ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                    ui.set_min_size(Vec2::new(96.0, 32.0));
                    ui.text_edit_singleline(&mut app.dialogs.add_metric_unit);
                });

                ui.add_space(8.0);

                // ---- Кнопка calendar-sync (Заглушка) ----
                // 32×32, bg: #2B2B2B, border-left: 1px #373839, SVG 15×15
                let cal_rect = egui::Rect::from_min_size(
                    ui.cursor().min,
                    Vec2::splat(32.0),
                );
                {
                    let p = ui.painter();
                    p.rect_filled(cal_rect, 0.0, BG_CALENDAR_STUB);
                    p.rect_stroke(
                        cal_rect,
                        0.0,
                        Stroke::new(1.0, BG_BORDER),
                        egui::StrokeKind::Middle,
                    );
                }
                ui.allocate_rect(cal_rect, egui::Sense::hover());
                lucide_button(ui, Lucide::CalendarSync, 15.0, COLOR_GRAY);
            });

            ui.add_space(12.0);

            // ---- Нижняя панель действий (Background) ----
            // flex-direction: row, justify-content: flex-end, align-items: center
            // padding: 0px 20px 20px, размер: 538×52, bg: #1E1E1E
            let bottom_rect = egui::Rect::from_min_size(
                dialog_rect.min + Vec2::new(1.0, 146.0),
                Vec2::new(538.0, 52.0),
            );
            {
                let p = ui.painter();
                p.rect_filled(bottom_rect, 0.0, BG_DIALOG);
            }
            ui.allocate_rect(bottom_rect, egui::Sense::hover());

            ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                ui.set_min_width(538.0);
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);

                // ---- Кнопка Отмена ----
                // bg: #FFFFFF, border: 1px #FFFFFF, border-radius: 4px, текст #1E1E1E
                let cancel_rect = egui::Rect::from_min_size(
                    ui.cursor().min,
                    Vec2::new(89.27, 32.0),
                );
                {
                    let p = ui.painter();
                    p.rect_filled(cancel_rect, 4.0, COLOR_WHITE);
                    p.rect_stroke(
                        cancel_rect,
                        4.0,
                        Stroke::new(1.0, COLOR_WHITE),
                        egui::StrokeKind::Middle,
                    );
                }
                ui.allocate_rect(cancel_rect, egui::Sense::click());

                if ui.button("Отмена").clicked() {
                    cancelled = true;
                }

                ui.add_space(8.0);

                // ---- Кнопка ОК ----
                if ui.button("ОК").clicked() {
                    done = true;
                }
            });
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

//! Модальное диалоговое окно добавления показателя.
//! Соответствует спецификации оформления: «Модальные окна - Оформление.md».
//! Размер диалога 540×198 px, заголовок с именем процесса,
//! переключение типа показателя по иконке (# для Number, Часы для Time и т.д.),
//! кнопка calendar-sync в виде заглушки (no-op).

use crate::app::PolygramApp;
use crate::app::icons::lucide_button;
use chrono::{Datelike, NaiveDate, NaiveTime, Utc};
use egui::{
    Align, Color32, Context, CornerRadius, Frame, Layout, Margin, Sense, Stroke, StrokeKind,
    Vec2, Window,
};
use egui_lucide::Lucide;
use polygram_core::{MetricKind, MetricValue, runtime::Published};

/// Цвет фона диалога: #1E1E1E.
const BG_DIALOG: Color32 = Color32::from_rgb(0x1E, 0x1E, 0x1E);
/// Цвет рамки: #373839.
const BG_BORDER: Color32 = Color32::from_rgb(0x37, 0x38, 0x39);
/// Цвет фона полей ввода: #252525.
const BG_INPUT: Color32 = Color32::from_rgb(0x25, 0x25, 0x25);
/// Цвет фона общей обёртки полей: #222222.
const BG_VALUE_ROW: Color32 = Color32::from_rgb(0x22, 0x22, 0x22);
/// Цвет фона кнопки calendar-sync: #2B2B2B.
const BG_CALENDAR_STUB: Color32 = Color32::from_rgb(0x2B, 0x2B, 0x2B);
/// Цвет белого: #FFFFFF.
const COLOR_WHITE: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
/// Цвет фона кнопки «Готово» при наведении: #E5E5E5.
const COLOR_BTN_HOVER: Color32 = Color32::from_rgb(0xE5, 0xE5, 0xE5);
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

    let mut done = false;
    let mut cancelled = false;

    let existing_md = published
        .metric_defs
        .iter()
        .find(|md| md.name == app.dialogs.add_metric_name.trim())
        .cloned();

    // Затемнённый фон позади модального окна (50%).
    let screen_rect = ctx.viewport_rect();
    ctx.layer_painter(egui::LayerId::background()).rect_filled(
        screen_rect,
        0.0,
        Color32::from_black_alpha(128),
    );

    // Центрирование модального окна по экрану.
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
        .frame(
            Frame::new()
                .fill(BG_DIALOG)
                .stroke(Stroke::new(1.0, BG_BORDER))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::same(0)),
        )
        .show(ctx, |ui| {
            // ---- Шапка диалога (padding: 20px 20px 12px) ----
            Frame::new()
                .fill(BG_DIALOG)
                .inner_margin(Margin { left: 20, top: 20, right: 20, bottom: 12 })
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(0.0, 0.0);

                        // ---- Контейнер имени процесса ----
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(0.0, 4.0);

                            // Имя процесса (Inter 600, 16px, #FFFFFF)
                            ui.label(
                                egui::RichText::new(&app.dialogs.add_metric_process_name)
                                    .strong()
                                    .size(16.0)
                                    .color(COLOR_WHITE),
                            );

                            // Строка типа показателя (gap 6px)
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(6.0, 0.0);

                                // Точка-индикатор (6×6, #A1A1AA, border-radius 3px)
                                let dot_center = ui.cursor().center();
                                {
                                    let p = ui.painter();
                                    p.circle_filled(dot_center, 3.0, COLOR_GRAY);
                                }
                                ui.add_space(6.0);

                                // Метка типа (ЗАПРОС / РЕЗУЛЬТАТ)
                                ui.label(
                                    egui::RichText::new(label)
                                        .strong()
                                        .small()
                                        .extra_letter_spacing(0.44)
                                        .text_style(egui::TextStyle::Small)
                                        .color(COLOR_GRAY),
                                );
                            });
                        });

                        // Кнопка закрытия в правом верхнем углу (28×28, border-radius 4px)
                        ui.add_space(277.84);
                        let x_resp = lucide_button(ui, Lucide::X, 16.0, COLOR_GRAY);
                        if x_resp.clicked() {
                            cancelled = true;
                        }
                    });
                });

            ui.add_space(4.0);

            // ---- Основное содержимое (padding: 4px 20px 16px) ----
            Frame::new()
                .fill(BG_DIALOG)
                .inner_margin(Margin { left: 20, top: 4, right: 20, bottom: 16 })
                .show(ui, |ui| {
                    // Background+Border: 498×52, bg #222222, border #373839, radius 4px, padding 8px, gap 8px
                    Frame::new()
                        .fill(BG_VALUE_ROW)
                        .stroke(Stroke::new(1.0, BG_BORDER))
                        .corner_radius(CornerRadius::same(4))
                        .inner_margin(Margin::symmetric(8, 8))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);

                                // ---- 1. Иконка типа (34×34, radius 4px) ----
                                Frame::new()
                                    .fill(BG_INPUT)
                                    .stroke(Stroke::new(1.0, BG_BORDER))
                                    .corner_radius(CornerRadius::same(4))
                                    .inner_margin(Margin::same(8))
                                    .show(ui, |ui| {
                                        ui.allocate_ui_with_layout(
                                            Vec2::new(16.0, 16.0),
                                            Layout::left_to_right(Align::Center),
                                            |ui| {
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
                                            },
                                        );
                                    });

                                ui.add_space(8.0);

                                // ---- 2. Имя показателя (204×34, radius 2px) ----
                                Frame::new()
                                    .fill(BG_INPUT)
                                    .stroke(Stroke::new(1.0, BG_BORDER))
                                    .corner_radius(CornerRadius::same(2))
                                    .inner_margin(Margin {
                                        left: 10,
                                        top: 8,
                                        right: 10,
                                        bottom: 8,
                                    })
                                    .show(ui, |ui| {
                                        ui.allocate_ui_with_layout(
                                            Vec2::new(182.0, 18.0),
                                            Layout::left_to_right(Align::TOP),
                                            |ui| {
                                                ui.add(
                                                    egui::TextEdit::singleline(
                                                        &mut app.dialogs.add_metric_name,
                                                    )
                                                    .frame(Frame::new())
                                                    .hint_text("Ввод имени Показателя"),
                                                );
                                            },
                                        );
                                    });

                                ui.add_space(8.0);

                                // ---- 3. Значение + кнопка синхронизации (130×34, одна рамка, radius 2px) ----
                                Frame::new()
                                    .fill(BG_INPUT)
                                    .stroke(Stroke::new(1.0, BG_BORDER))
                                    .corner_radius(CornerRadius::same(2))
                                    .inner_margin(Margin::same(0))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

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
                                                    (Some(d), None) => {
                                                        d.format("%d.%m.%Y").to_string()
                                                    }
                                                    (None, Some(t)) => {
                                                        "дата ".to_owned()
                                                            + &t.format("%H:%M").to_string()
                                                    }
                                                    _ => "дата".to_string(),
                                                };
                                                if ui.button(date_label).clicked() {
                                                    app.dialogs.add_metric_date_popup_open = true;
                                                }
                                                if app.dialogs.add_metric_date_popup_open {
                                                    let mut picked = false;
                                                    egui::Window::new("Выберите дату")
                                                        .collapsible(false)
                                                        .resizable(false)
                                                        .show(ui.ctx(), |ui| {
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
                                                    (Some(d), None) => {
                                                        d.format("%d.%m.%Y").to_string()
                                                    }
                                                    _ => String::new(),
                                                };
                                            } else {
                                                // Поле значения (94×32 внутри Unified frame)
                                                Frame::new()
                                                    .fill(BG_INPUT)
                                                    .stroke(Stroke::new(1.0, BG_BORDER))
                                                    .corner_radius(CornerRadius::same(2))
                                                    .inner_margin(Margin {
                                                        left: 8,
                                                        top: 7,
                                                        right: 8,
                                                        bottom: 7,
                                                    })
                                                    .show(ui, |ui| {
                                                        ui.allocate_ui_with_layout(
                                                            Vec2::new(76.0, 18.0),
                                                            Layout::left_to_right(Align::TOP),
                                                            |ui| {
                                                                ui.add(
                                                                    egui::TextEdit::singleline(
                                                                        &mut app
                                                                            .dialogs
                                                                            .add_metric_value,
                                                                    )
                                                                    .frame(Frame::new())
                                                                    .hint_text("Ввод значения"),
                                                                );
                                                            },
                                                        );
                                                    });
                                            }

                                            ui.add_space(4.0);

                                            // Кнопка синхронизации (32×32)
                                            Frame::new()
                                                .fill(BG_CALENDAR_STUB)
                                                .show(ui, |ui| {
                                                    ui.allocate_ui_with_layout(
                                                        Vec2::new(32.0, 32.0),
                                                        Layout::left_to_right(Align::Center),
                                                        |ui| {
                                                            lucide_button(
                                                                ui,
                                                                Lucide::CalendarSync,
                                                                15.0,
                                                                COLOR_GRAY,
                                                            );
                                                        },
                                                    );
                                                });
                                        });
                                    });

                                ui.add_space(8.0);

                                // ---- 4. Единица измерения (88×34, radius 2px) ----
                                Frame::new()
                                    .fill(BG_INPUT)
                                    .stroke(Stroke::new(1.0, BG_BORDER))
                                    .corner_radius(CornerRadius::same(2))
                                    .inner_margin(Margin {
                                        left: 10,
                                        top: 8,
                                        right: 10,
                                        bottom: 8,
                                    })
                                    .show(ui, |ui| {
                                        ui.allocate_ui_with_layout(
                                            Vec2::new(66.0, 18.0),
                                            Layout::right_to_left(Align::TOP),
                                            |ui| {
                                                ui.add(
                                                    egui::TextEdit::singleline(
                                                        &mut app.dialogs.add_metric_unit,
                                                    )
                                                    .frame(Frame::new())
                                                    .hint_text("Ввод единицы измерения"),
                                                );
                                            },
                                        );
                                    });
                            });
                        });
                });

            ui.add_space(12.0);

            // ---- Нижняя панель действий ----
            Frame::new()
                .fill(BG_DIALOG)
                .inner_margin(Margin { left: 0, top: 0, right: 20, bottom: 20 })
                .show(ui, |ui| {
                    ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);

                        // Кнопка Готово (#FFFFFF, при наведении #E5E5E5, border #FFFFFF, radius 4px, текст #1E1E1E, невыделяемый)
                        let btn_size = Vec2::new(89.27, 32.0);
                        let (btn_rect, btn_response) = ui.allocate_exact_size(btn_size, Sense::click());
                        let btn_hovered = btn_response.hovered();
                        let btn_fill = if btn_hovered { COLOR_BTN_HOVER } else { COLOR_WHITE };

                        ui.painter().rect_filled(btn_rect, CornerRadius::same(4), btn_fill);
                        ui.painter().rect_stroke(
                            btn_rect,
                            CornerRadius::same(4),
                            Stroke::new(1.0, COLOR_WHITE),
                            StrokeKind::Middle,
                        );

                        let font_id = egui::TextStyle::Button.resolve(ui.style());
                        let galley = ui.fonts_mut(|f| f.layout_no_wrap("Готово".to_owned(), font_id, BG_DIALOG));
                        let text_pos = btn_rect.center() - galley.size() / 2.0;
                        ui.painter().galley(text_pos, galley, BG_DIALOG);

                        if btn_response.clicked() {
                            done = true;
                        }
                        ui.add_space(32.0);
                    });
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

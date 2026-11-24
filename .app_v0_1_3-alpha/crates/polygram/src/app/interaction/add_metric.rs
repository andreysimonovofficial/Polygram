//! Модальное диалоговое окно добавления показателя.
//! Соответствует спецификации оформления: «Модальные окна - Оформление.md».
//! Универсальная оболочка (оверлей, рамка, шапка, подвал, состояния)
//! вынесена в `polygram_canvas::modal`; здесь осталось только специфическое
//! тело окна добавления показателя.

use crate::app::PolygramApp;
use polygram_canvas::modal::{
    footer, header, input_field, modal_frame, paint_overlay,
    MODAL_BG, MODAL_BORDER, MODAL_INPUT_BG, MODAL_INPUT_HOVER_BG,
    MODAL_INPUT_HOVER_BORDER, MODAL_TEXT, MODAL_TEXT_MUTED, MODAL_TYPE_ACTIVE_BG,
    MODAL_TYPE_HOVER_BG,
};
use polygram_canvas::paint_lucide;
use chrono::{Datelike, NaiveDate, NaiveTime, Utc};
use egui::{
    Align, Color32, Context, CornerRadius, Frame, Layout, Margin, PointerButton, Rect, Sense,
    Stroke, StrokeKind, Ui, Vec2, Window,
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

            ui.add_space(4.0);

            // ---- Тело диалога (отступы 4/20/20/16) ----
            Frame::new()
                .fill(MODAL_BG)
                .inner_margin(Margin {
                    left: 20,
                    top: 4,
                    right: 20,
                    bottom: 16,
                })
                .show(ui, |ui| {
                    // Внутренняя рамка: 498×52, bg #222222, border #373839, radius 4, padding 8, gap 8
                    let inner_size = Vec2::new(498.0, 52.0);
                    let (inner_rect, _) = ui.allocate_exact_size(inner_size, Sense::hover());
                    ui.painter().rect_filled(
                        inner_rect,
                        CornerRadius::same(4),
                        BG_VALUE_ROW,
                    );
                    ui.painter().rect_stroke(
                        inner_rect,
                        CornerRadius::same(4),
                        Stroke::new(1.0, MODAL_BORDER),
                        StrokeKind::Middle,
                    );

                    const PAD: f32 = 8.0;
                    let work = Rect::from_min_max(
                        egui::pos2(inner_rect.min.x + PAD, inner_rect.min.y + PAD),
                        egui::pos2(
                            inner_rect.max.x - PAD,
                            inner_rect.max.y - PAD,
                        ),
                    );
                    let mut row = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(work)
                            .layout(Layout::left_to_right(Align::Center)),
                    );
                    row.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);
                    row.horizontal(|ui| {
                        // ---- 1. Кнопка смены типа показателя (34×34, radius 4) ----
                        let type_size = Vec2::splat(34.0);
                        let (type_rect, type_response) =
                            ui.allocate_exact_size(type_size, Sense::click());
                        let type_hovered = type_response.hovered();
                        let type_pressed = type_hovered
                            && ui.ctx().input(|i| i.pointer.button_down(PointerButton::Primary));
                        let type_fill = if type_pressed {
                            MODAL_TYPE_ACTIVE_BG
                        } else if type_hovered {
                            MODAL_TYPE_HOVER_BG
                        } else {
                            MODAL_INPUT_BG
                        };
                        ui.painter()
                            .rect_filled(type_rect, CornerRadius::same(4), type_fill);
                        ui.painter().rect_stroke(
                            type_rect,
                            CornerRadius::same(4),
                            Stroke::new(1.0, MODAL_BORDER),
                            StrokeKind::Middle,
                        );
                        let icon_rect =
                            Rect::from_center_size(type_rect.center(), Vec2::splat(16.0));
                        paint_lucide(ui, icon_rect, metric_kind_icon(app.dialogs.add_metric_kind), MODAL_TEXT);
                        if type_response.clicked() {
                            let kind = metric_kind_next(app.dialogs.add_metric_kind);
                            app.dialogs.add_metric_kind = kind;
                            app.dialogs.add_metric_value.clear();
                            app.dialogs.add_metric_date = None;
                            app.dialogs.add_metric_date_popup_open = false;
                            app.dialogs.add_metric_time = None;
                        }

                        // ---- 2. Имя показателя (растягивается, высота 34, radius 2) ----
                        let name_w = work.width() - 34.0 - 130.0 - 88.0 - 3.0 * 8.0;
                        input_field(
                            ui,
                            &mut app.dialogs.add_metric_name,
                            "Имя Показателя",
                            Vec2::new(name_w, 34.0),
                            false,
                            Margin::symmetric(10, 8),
                        );

                        // ---- 3. Составной блок значение + синхронизация (130×34, radius 2) ----
                        let vs_size = Vec2::new(130.0, 34.0);
                        let (vs_rect, _) = ui.allocate_exact_size(vs_size, Sense::hover());
                        ui.painter()
                            .rect_filled(vs_rect, CornerRadius::same(2), MODAL_INPUT_BG);
                        ui.painter().rect_stroke(
                            vs_rect,
                            CornerRadius::same(2),
                            Stroke::new(1.0, MODAL_BORDER),
                            StrokeKind::Middle,
                        );
                        let vs_work = Rect::from_min_max(
                            egui::pos2(vs_rect.min.x + 1.0, vs_rect.min.y + 1.0),
                            egui::pos2(vs_rect.max.x - 1.0, vs_rect.max.y - 1.0),
                        );
                        let mut vs = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(vs_work)
                                .layout(Layout::left_to_right(Align::Center)),
                        );
                        vs.spacing_mut().item_spacing = Vec2::new(0.0, 0.0);
                        vs.horizontal(|ui| {
                            // Поле ввода значения: 96×32, правое выравнивание, плейсхолдер «Значение».
                            if is_date_kind(app.dialogs.add_metric_kind) {
                                render_date_button(
                                    ui,
                                    &mut app.dialogs.add_metric_date,
                                    &mut app.dialogs.add_metric_time,
                                    &mut app.dialogs.add_metric_date_popup_open,
                                    &mut app.dialogs.add_metric_value,
                                    ctx,
                                );
                            } else {
                                input_field(
                                    ui,
                                    &mut app.dialogs.add_metric_value,
                                    "Значение",
                                    Vec2::new(96.0, 32.0),
                                    true,
                                    Margin::symmetric(8, 8),
                                );
                            }

                            // Кнопка синхронизации (32×32, левая граница 1px #373839, заглушка).
                            let sync_size = Vec2::splat(32.0);
                            let (sync_rect, sync_response) =
                                ui.allocate_exact_size(sync_size, Sense::click());
                            let sync_hovered = sync_response.hovered();
                            let sync_pressed = sync_hovered
                                && ui.ctx().input(|i| i.pointer.button_down(PointerButton::Primary));
                            let sync_fill = if sync_pressed {
                                BG_SYNC_ACTIVE
                            } else if sync_hovered {
                                BG_SYNC_HOVER
                            } else {
                                BG_SYNC_BASE
                            };
                            ui.painter()
                                .rect_filled(sync_rect, CornerRadius::same(0), sync_fill);
                            // Левая граница-разделитель 1 px #373839.
                            ui.painter().line_segment(
                                [
                                    sync_rect.min,
                                    sync_rect.min + egui::vec2(0.0, 32.0),
                                ],
                                Stroke::new(1.0, MODAL_BORDER),
                            );
                            let icon_rect =
                                Rect::from_center_size(sync_rect.center(), Vec2::splat(15.0));
                            paint_lucide(ui, icon_rect, Lucide::CalendarSync, MODAL_TEXT_MUTED);
                            // Кнопка является визуальной заглушкой: при нажатии ничего не происходит.
                            let _ = sync_response;
                        });

                        // ---- 4. Единица измерения (88×34, правое выравнивание, radius 2) ----
                        input_field(
                            ui,
                            &mut app.dialogs.add_metric_unit,
                            "Ед.",
                            Vec2::new(88.0, 34.0),
                            true,
                            Margin::symmetric(10, 8),
                        );
                    });
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

/// Является ли тип показателя датовым (Date / Deadline).
fn is_date_kind(kind: MetricKind) -> bool {
    matches!(kind, MetricKind::Date | MetricKind::Deadline)
}

/// Кнопка выбора даты/времени для датовых типов (96×32, оформлена как поле ввода).
fn render_date_button(
    ui: &mut Ui,
    selected_date: &mut Option<NaiveDate>,
    selected_time: &mut Option<NaiveTime>,
    popup_open: &mut bool,
    value_buf: &mut String,
    ctx: &Context,
) {
    let size = Vec2::new(96.0, 32.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let hovered = response.hovered();
    let pressed = hovered && ui.ctx().input(|i| i.pointer.button_down(PointerButton::Primary));
    let fill = if hovered || pressed {
        MODAL_INPUT_HOVER_BG
    } else {
        MODAL_INPUT_BG
    };
    let stroke = if hovered || pressed {
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

    let date_label = match (*selected_date, *selected_time) {
        (Some(d), Some(t)) => d.format("%d.%m.%Y").to_string() + " " + &t.format("%H:%M").to_string(),
        (Some(d), None) => d.format("%d.%m.%Y").to_string(),
        (None, Some(t)) => "дата ".to_owned() + &t.format("%H:%M").to_string(),
        _ => "дата".to_string(),
    };
    if response.clicked() {
        *popup_open = true;
    }
    if *popup_open {
        let mut picked = false;
        egui::Window::new("Выберите дату")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                if render_date_time_popup(ui, selected_date, selected_time) {
                    picked = true;
                }
            });
        if picked {
            *popup_open = false;
        }
    }
    *value_buf = match (*selected_date, *selected_time) {
        (Some(d), Some(_t)) => d.format("%d.%m.%Y %H:%M").to_string(),
        (Some(d), None) => d.format("%d.%m.%Y").to_string(),
        _ => String::new(),
    };

    let font_id = egui::FontId::new(14.0, egui::FontFamily::Proportional);
    let galley = ui
        .fonts_mut(|f| f.layout_no_wrap(date_label, font_id, MODAL_TEXT));
    let text_x = rect.right() - 8.0 - galley.size().x;
    let text_y = rect.center().y - galley.size().y / 2.0;
    let text_pos = egui::pos2(text_x, text_y);
    ui.painter().galley(text_pos, galley, MODAL_TEXT);
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

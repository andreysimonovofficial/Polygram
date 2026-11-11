use crate::app::PolygramApp;
use chrono::{Datelike, NaiveDate, NaiveTime, Timelike, Utc};
use polygram_core::{Command, MetricKind, MetricValue, Published};
use egui::{Context, Window};

/// Модальное диалоговое окно добавления показателя.
pub fn handle_add_metric_dialog(
    app: &mut PolygramApp,
    ctx: &Context,
    published: &Published,
) {
    let Some((object_id, dir)) = &app.dialogs.add_metric_target else {
        return;
    };
    let object_id = object_id.clone();
    let dir = *dir;

    let label = match dir {
        polygram_core::types::Direction::Request => "ЗАПРОС",
        polygram_core::types::Direction::Output => "ВЫХОД",
    };
    let value_label = match dir {
        polygram_core::types::Direction::Request => "Необходимое (required):",
        polygram_core::types::Direction::Output => "Значение (value):",
    };

    let mut done = false;
    let mut cancelled = false;

    // Ищем существующий показатель по введённому имени (для подстановки данных).
    let existing_md = published
        .metric_defs
        .iter()
        .find(|md| md.name == app.dialogs.add_metric_name.trim())
        .cloned();

    Window::new(format!("Добавить показатель — {label}"))
        .default_size([320.0, 240.0])
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label("Имя показателя:");
            ui.text_edit_singleline(&mut app.dialogs.add_metric_name);

            ui.separator();
            ui.label("Тип:");
            egui::ComboBox::new("add_metric_kind", "")
                .selected_text(format!("{:?}", app.dialogs.add_metric_kind))
                .show_ui(ui, |ui| {
                    for kind in [
                        MetricKind::Number,
                        MetricKind::Time,
                        MetricKind::Date,
                        MetricKind::Deadline,
                        MetricKind::Percentage,
                        MetricKind::Binary,
                        MetricKind::Duration,
                        MetricKind::Money,
                        MetricKind::Text,
                        MetricKind::Formula,
                    ] {
                        if ui.selectable_label(app.dialogs.add_metric_kind == kind, format!("{:?}", kind)).clicked() {
                            app.dialogs.add_metric_kind = kind;
                            app.dialogs.add_metric_value.clear();
                            app.dialogs.add_metric_date = None;
                            app.dialogs.add_metric_date_popup_open = false;
                            app.dialogs.add_metric_time = None;
                        }
                    }
                });

            if let Some(md) = &existing_md {
                ui.label(format!("Найден существующий показатель: {}", md.name));
                ui.label(format!("Тип: {:?}", md.kind));
                ui.label(format!("Единица: {}", md.unit));
            }

            ui.separator();
            ui.label(value_label);
            let is_date_kind = matches!(
                app.dialogs.add_metric_kind,
                MetricKind::Date | MetricKind::Deadline
            );
            if is_date_kind {
                let date_label = match (
                    app.dialogs.add_metric_date,
                    app.dialogs.add_metric_time,
                ) {
                    (Some(d), Some(t)) => d.format("%d.%m.%Y").to_string()
                        + " "
                        + &t.format("%H:%M").to_string(),
                    (Some(d), None) => d.format("%d.%m.%Y").to_string(),
                    (None, Some(t)) => "дата ".to_owned() + &t.format("%H:%M").to_string(),
                    _ => "дата".to_owned(),
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
                    (Some(d), Some(t)) => d.format("%d.%m.%Y %H:%M").to_string(),
                    (Some(d), None) => d.format("%d.%m.%Y").to_string(),
                    _ => String::new(),
                };
            } else {
                ui.text_edit_singleline(&mut app.dialogs.add_metric_value);
            }

            ui.separator();
            ui.label("Единица измерения:");
            ui.text_edit_singleline(&mut app.dialogs.add_metric_unit);

            ui.separator();
            ui.heading("Синхронизация с API (необязательно)");
            let connectors = app.connectors.get_connectors();
            let current_connector = &mut app.dialogs.add_metric_api_connector;
            egui::ComboBox::new("add_api_connector", "")
                .selected_text(if current_connector.is_empty() {
                    "коннектор".to_owned()
                } else {
                    current_connector.clone()
                })
                .show_ui(ui, |ui| {
                    if connectors.is_empty() {
                        ui.disable();
                        ui.label("нет коннекторов (API недоступен)");
                    } else {
                        for c in &connectors {
                            if ui.selectable_label(current_connector == c, c).clicked() {
                                *current_connector = c.clone();
                            }
                        }
                    }
                });
            ui.label("ячейка / метрика:");
            ui.text_edit_singleline(&mut app.dialogs.add_metric_api_cell);

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    done = true;
                }
                if ui.button("Отмена").clicked() {
                    cancelled = true;
                }
            });
        });

    if cancelled {
        app.dialogs.add_metric_target = None;
        app.dialogs.add_metric_name.clear();
        app.dialogs.add_metric_value.clear();
        app.dialogs.add_metric_date = None;
        app.dialogs.add_metric_date_popup_open = false;
        app.dialogs.add_metric_time = None;
        app.dialogs.add_metric_unit.clear();
        app.dialogs.add_metric_api_connector.clear();
        app.dialogs.add_metric_api_cell.clear();
        return;
    }

    if done {
        let name = app.dialogs.add_metric_name.trim().to_owned();
        if name.is_empty() {
            app.dialogs.add_metric_target = None;
            app.dialogs.add_metric_name.clear();
            app.dialogs.add_metric_value.clear();
            app.dialogs.add_metric_unit.clear();
            return;
        }

        let value: MetricValue = parse_metric_value(&app.dialogs.add_metric_value, app.dialogs.add_metric_kind);
        let object_id_for_cmd = object_id.clone();
        let name_for_cmd = name.clone();

        let kind = app.dialogs.add_metric_kind;
        let unit = app.dialogs.add_metric_unit.trim().to_owned();
        let (entry_value, entry_required) = match dir {
            polygram_core::types::Direction::Request => (MetricValue::Null, value.clone()),
            polygram_core::types::Direction::Output => (value.clone(), MetricValue::Null),
        };

        let reuse_existing = existing_md
            .as_ref()
            .map(|md| kind == md.kind)
            .unwrap_or(false);

        if reuse_existing {
            // Используем существующий показатель — создаём только запись на объекте.
            let md = existing_md.as_ref().unwrap();
            app.apply_batch(vec![Command::AddMetricEntry {
                object: object_id_for_cmd.clone(),
                dir,
                id: Some(name_for_cmd.clone()),
                metric_def: md.name.clone(),
                name: name_for_cmd.clone(),
                value: entry_value,
                required: entry_required,
                max: None,
                fact: MetricValue::Null,
            }]);
        } else {
            app.apply_batch(vec![
                Command::AddMetricDef {
                    id: Some(name_for_cmd.clone()),
                    name: name_for_cmd.clone(),
                    kind,
                    unit,
                    api: None,
                },
                Command::AddMetricEntry {
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

            // API-привязка нового показателя.
            if !app.dialogs.add_metric_api_connector.is_empty()
                && !app.dialogs.add_metric_api_cell.is_empty()
            {
                app.post_binding(
                    object_id_for_cmd.clone(),
                    name_for_cmd.clone(),
                    app.dialogs.add_metric_api_connector.clone(),
                    app.dialogs.add_metric_api_cell.clone(),
                    30,
                );
            }
        }

        // API-привязка к существующему показателю.
        if !app.dialogs.add_metric_api_connector.is_empty()
            && !app.dialogs.add_metric_api_cell.is_empty()
        {
            app.post_binding(
                object_id_for_cmd.clone(),
                name_for_cmd.clone(),
                app.dialogs.add_metric_api_connector.clone(),
                app.dialogs.add_metric_api_cell.clone(),
                30,
            );
        }

        app.dialogs.add_metric_target = None;
        app.dialogs.add_metric_name.clear();
        app.dialogs.add_metric_kind = MetricKind::Number;
        app.dialogs.add_metric_value.clear();
        app.dialogs.add_metric_date = None;
        app.dialogs.add_metric_date_popup_open = false;
        app.dialogs.add_metric_time = None;
        app.dialogs.add_metric_unit.clear();
        app.dialogs.add_metric_api_connector.clear();
        app.dialogs.add_metric_api_cell.clear();
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
/// Использует egui::Grid, чтобы колонки дней недели и даты были ровно выровнены.
/// Возвращает true, если в этом кадре была выбрана конкретная дата.
fn render_date_time_popup(
    ui: &mut egui::Ui,
    selected_date: &mut Option<NaiveDate>,
    selected_time: &mut Option<NaiveTime>,
) -> bool {
    let now = chrono::Local::now();
    let show_date = selected_date.unwrap_or(now.date_naive());
    let show_time = selected_time.unwrap_or(now.time());

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
        // навигация месяца (занимает всю ширину сетки)
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

        // заголовки дней недели
        for wd in ["Пн", "Вт", "Ср", "Чт", "Пт", "Сб", "Вс"] {
            ui.label(wd);
        }
        ui.end_row();

        // сетка дней
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

    // ввод времени (чч:мм)
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

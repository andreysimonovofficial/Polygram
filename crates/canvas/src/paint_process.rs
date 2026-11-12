//! Процедурный рендер объекта "Процесс" (конструктор-style, без design.json).
//!
//! Все константы в начале файла — крути их, результат меняется сразу.
//! Высота объекта растёт под контент: заголовок + 2 × (заголовок контейнера + строки показателей).
//! Всё считается из констант: нет наложения статики и динамики, нет двойной отрисовки.

use crate::theme::Theme;
use egui::{Color32, Painter, Pos2, Rect, Stroke, Vec2};
use polygram_core::types::Direction;
use polygram_core::{DynamicState, EntryGeom, ObjectGeom, StatusCode};
use std::collections::HashMap;

// ┌─────────────────────────────────────────────────────────────┐
// │ КОНСТАНТЫ-КОНСТРУКТОР                                       │
// └─────────────────────────────────────────────────────────────┘

/// Максимум видимых показателей на сторону (лимит UI, не ядра).
pub const MAX_VISIBLE_METRICS: usize = 4;

/// Контур строки показателя.
pub const ROW_BORDER_WIDTH: f32 = 1.0;

/// Зазор между соседними строками показателя.
pub const ROW_GAP_PX: f32 = 4.0;

/// Радиус точки сцепления строки показателя (круг, не символ).
pub const CONNECTION_DOT_RADIUS: f32 = 4.0;

/// Отступ от края строки показателя до символа сцепления (▶/◀).
const SYMBOL_X_PAD: f32 = 8.0;
/// Отступ от края строки показателя до текста (имя / значение + единицы).
const TEXT_X_PAD: f32 = 20.0;

/// Минимальная высота строки показателя (пиксели при zoom = 1).
pub const MIN_ROW_H_PX: f32 = 27.0;

/// Информация о API-привязке одной строки показателя (показывается внутри строки).
#[derive(Clone, Debug, Default)]
pub struct MetricBinding {
    pub ok: bool,
    pub last_updated: Option<String>,
    pub error: Option<String>,
}

/// Ранг статуса (чем выше — тем хуже).
fn status_rank(s: StatusCode) -> u8 {
    match s {
        StatusCode::Ok => 0,
        StatusCode::Unknown | StatusCode::Disabled => 1,
        StatusCode::Unconnected => 2,
        StatusCode::Warning => 3,
        StatusCode::Deficit => 4,
        StatusCode::Surplus => 5,
        StatusCode::Invalid => 6,
    }
}

/// Худший статус среди записей.
fn worst_status(entries: &[polygram_core::EntryGeom], dynamic: &DynamicState) -> StatusCode {
    let mut worst: u8 = 0;
    for e in entries {
        let r = status_rank(dynamic.status[e.ix as usize]);
        if r > worst { worst = r; }
    }
    match worst {
        4 => StatusCode::Deficit,
        5 => StatusCode::Surplus,
        3 => StatusCode::Warning,
        2 => StatusCode::Unconnected,
        6 => StatusCode::Invalid,
        _ => StatusCode::Ok,
    }
}

/// Рисовать одну строку показателя: точка сцепления слева и справа, имя, значение справа.
/// Для запроса показывается required, для выхода — value.
/// binding — опциональная API-привязка этой строки (иконка + время последнего опроса внутри строки).
/// error_hotspots — накопление rect иконок ошибок + текст ошибки для копирования по клику.
fn paint_metric_row(
    painter: &Painter,
    rect: Rect,
    entry: &polygram_core::EntryGeom,
    dynamic: &DynamicState,
    zoom: f32,
    theme: &Theme,
    is_hovered: bool,
    dir: Direction,
    binding: Option<&MetricBinding>,
    error_hotspots: &mut Vec<(Rect, String)>,
) {
    let status = dynamic.status[entry.ix as usize];
    let cy = rect.center().y;

    // Контур всей строки
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(ROW_BORDER_WIDTH * zoom, theme.card_border),
    );

    // Символы сцепления слева и справа внутри тела строки.
    // Запрос: слева ▶, справа ◀. Выход: слева ◀, справа ▶.
    let dot_color = super::status_dot_color(status, dir, theme);
    let symbol_font = egui::FontId::new((theme.text_size * zoom).max(10.0 * zoom), egui::FontFamily::Proportional);
    let (left_sym, right_sym) = match dir {
        Direction::Request => ("▶", "◀"),
        Direction::Output => ("◀", "▶"),
    };
    painter.text(
        Pos2::new(rect.min.x + SYMBOL_X_PAD * zoom, cy),
        egui::Align2::LEFT_CENTER,
        left_sym,
        symbol_font.clone(),
        dot_color,
    );
    painter.text(
        Pos2::new(rect.max.x - SYMBOL_X_PAD * zoom, cy),
        egui::Align2::RIGHT_CENTER,
        right_sym,
        symbol_font,
        dot_color,
    );

    // Имя записи
    let name_color = if is_hovered { theme.text } else { theme.text_muted };
    painter.text(
        Pos2::new(rect.min.x + TEXT_X_PAD * zoom, cy),
        egui::Align2::LEFT_CENTER,
        &entry.name,
        egui::FontId::new(theme.text_size * zoom, egui::FontFamily::Proportional),
        name_color,
    );

    // Иконка привязки + время последнего опроса (внутри строки, масштабируются с zoom).
    if let Some(b) = binding {
        let icon_color = if b.ok {
            egui::Color32::from_rgb(0, 200, 80)
        } else {
            egui::Color32::from_rgb(240, 80, 80)
        };
        let icon_text = if b.ok { "🌐" } else { "⚠" };
        let name_w_est = entry.name.len() as f32 * 7.0 * zoom;
        let icon_x = rect.min.x + TEXT_X_PAD * zoom + name_w_est + 8.0 * zoom;
        painter.text(
            Pos2::new(icon_x, cy),
            egui::Align2::LEFT_CENTER,
            icon_text,
            egui::FontId::new(theme.text_size * zoom, egui::FontFamily::Proportional),
            icon_color,
        );
        if let Some(ts) = &b.last_updated {
            painter.text(
                Pos2::new(icon_x + 14.0 * zoom, cy),
                egui::Align2::LEFT_CENTER,
                ts,
                egui::FontId::new(theme.text_size * zoom, egui::FontFamily::Proportional),
                egui::Color32::from_gray(170),
            );
        }
        // Кликабельная зона иконки ошибки — копировать текст ошибки по клику.
        if let Some(e) = &b.error {
            if !b.ok {
                let icon_rect = Rect::from_min_size(
                    Pos2::new(icon_x - 2.0 * zoom, cy - 12.0 * zoom),
                    Vec2::new(28.0 * zoom, 24.0 * zoom),
                );
                error_hotspots.push((icon_rect, e.clone()));
            }
        }
    }

    // Значение (справа): для запроса — required, для выхода — value.
    let unit = if entry.unit.is_empty() {
        String::new()
    } else {
        format!(" {}", entry.unit)
    };
    let displayed_value = match dir {
        Direction::Request => dynamic.required[entry.ix as usize].clone(),
        Direction::Output => entry.value.clone(),
    };
    let value_text = format!("{:.0}{}", displayed_value, unit);
    painter.text(
        Pos2::new(rect.max.x - TEXT_X_PAD * zoom, cy),
        egui::Align2::RIGHT_CENTER,
        &value_text,
        egui::FontId::new(theme.text_size * zoom, egui::FontFamily::Proportional),
        name_color,
    );
}

/// Рисовать графическую копию строки показателя в произвольном месте (превью при Drag&Drop).
/// То же оформление, что и обычная строка: рамка, точки сцепления, имя, иконка привязки, значение.
pub fn paint_drag_preview(
    painter: &Painter,
    rect: Rect,
    entry: &EntryGeom,
    dynamic: &DynamicState,
    zoom: f32,
    theme: &Theme,
    dir: Direction,
    binding: Option<&MetricBinding>,
) {
    paint_metric_row(painter, rect, entry, dynamic, zoom, theme, false, dir, binding, &mut Vec::new());
}

/// Пустая пунктирная рамка ("призрачная копия") размером с объект.
/// Рисуется только обводка пунктиром без заливки и содержимого.
pub fn paint_dashed_rect(painter: &Painter, rect: Rect, zoom: f32, color: Color32) {
    let dash = (6.0 * zoom).max(1.0);
    let gap = (4.0 * zoom).max(1.0);
    let stroke = Stroke::new((2.0 * zoom).max(1.0), color);
    let mut x = rect.min.x;
    while x < rect.max.x {
        let x2 = (x + dash).min(rect.max.x);
        painter.line_segment([Pos2::new(x, rect.min.y), Pos2::new(x2, rect.min.y)], stroke);
        painter.line_segment([Pos2::new(x, rect.max.y), Pos2::new(x2, rect.max.y)], stroke);
        x = x2 + gap;
    }
    let mut y = rect.min.y;
    while y < rect.max.y {
        let y2 = (y + dash).min(rect.max.y);
        painter.line_segment([Pos2::new(rect.min.x, y), Pos2::new(rect.min.x, y2)], stroke);
        painter.line_segment([Pos2::new(rect.max.x, y), Pos2::new(rect.max.x, y2)], stroke);
        y = y2 + gap;
    }
}

/// Рисовать строку "+ Добавить показатель" (центрированная, кликабельная).
fn paint_add_row(
    painter: &Painter,
    rect: Rect,
    zoom: f32,
    theme: &Theme,
    is_hovered: bool,
) {
    let add_text = "+ Добавить показатель";
    let add_color = if is_hovered { theme.text } else { theme.text_muted };
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        add_text,
        egui::FontId::new(theme.text_size * zoom, egui::FontFamily::Proportional),
        add_color,
    );
}

/// Рисовать заголовок контейнера: «ЗАПРОС (N) СТАТУС (N)» + точка статуса + кнопка ≡.
/// Возвращает rect кнопки ≡ (если нужно обрабатывать клик).
fn paint_container_header(
    painter: &Painter,
    rect: Rect,
    dir: Direction,
    entries: &[polygram_core::EntryGeom],
    dynamic: &DynamicState,
    zoom: f32,
    theme: &Theme,
    is_hovered: bool,
) -> Option<Rect> {
    let label = match dir {
        Direction::Request => "ЗАПРОС",
        Direction::Output => "ВЫХОД",
    };
    let status = worst_status(entries, dynamic);
    let status_text = super::status_label_ru(status, dir);
    let problem_count = entries.iter().filter(|e| dynamic.status[e.ix as usize] != StatusCode::Ok).count();
    let status_full = if problem_count > 0 {
        format!("{} ({})", status_text, problem_count)
    } else {
        status_text.to_string()
    };
    let count_text = format!("{} ({}) {}", label, entries.len(), status_full);
    let header_color = if status != StatusCode::Ok {
        super::status_dot_color(status, dir, theme)
    } else {
        theme.text_muted
    };
    let text_color = if is_hovered { theme.text } else { header_color };

    // Точка статуса контейнера (слева)
    let dot_color = super::status_dot_color(status, dir, theme);
    let dot_radius = theme.port_radius.max(2.0) * zoom;
    painter.circle_filled(
        Pos2::new(rect.min.x + 8.0 * zoom, rect.center().y),
        dot_radius,
        dot_color,
    );

    // Кнопка ≡ (справа)
    let btn_size = 24.0 * zoom;
    let btn_rect = Rect::from_min_size(
        rect.max - Vec2::new(btn_size, rect.height()),
        Vec2::new(btn_size, rect.height()),
    );
    painter.rect_stroke(btn_rect, 0.0, Stroke::new(1.0 * zoom, theme.card_border));
    let btn_color = if is_hovered { theme.text } else { header_color };
    painter.text(
        btn_rect.center(),
        egui::Align2::CENTER_CENTER,
        "≡",
        egui::FontId::new(theme.text_size * zoom, egui::FontFamily::Proportional),
        btn_color,
    );

    // Текст заголовка (с усечением если не влазит)
    let text_w_est = count_text.len() as f32 * 7.0 * zoom;
    let avail_w = (btn_rect.min.x - rect.min.x - 32.0 * zoom).max(0.0);
    let display = if text_w_est > avail_w && avail_w > 14.0 * zoom {
        let n = (avail_w / (7.0 * zoom)) as usize;
        if n > 4 {
            count_text.chars().take(n - 4).collect::<String>() + "..."
        } else {
            "...".to_string()
        }
    } else {
        count_text.clone()
    };
    painter.text(
        Pos2::new(rect.min.x + 16.0 * zoom, rect.center().y),
        egui::Align2::LEFT_CENTER,
        &display,
        egui::FontId::new(theme.text_size * zoom, egui::FontFamily::Proportional),
        text_color,
    );

    Some(btn_rect)
}

/// Высокопроизводительный процедурный рендер объекта "Процесс".
/// Рисует: внешняя рамка → заголовок объекта → контейнер Запрос → контейнер Выход.
/// Возвращает (горячие зоны добавления показателя, горячие зоны меню контейнеров, горячие зоны перетаскивания показателей).
pub fn paint_process(
    painter: &Painter,
    object: &ObjectGeom,
    dynamic: &DynamicState,
    theme: &Theme,
    rect_px: Rect,
    _cell_px: f32,
    zoom: f32,
    is_hovered: bool,
    _edit_target: Option<&str>,
    bindings: &HashMap<(String, String), MetricBinding>,
    error_hotspots: &mut Vec<(Rect, String)>,
) -> (
    Vec<(String, Direction, Rect)>,
    Vec<(String, Direction, Rect)>,
    Vec<(String, Direction, String, Rect)>,
) {
    let mut add_hotspots: Vec<(String, Direction, Rect)> = Vec::new();
    let mut menu_hotspots: Vec<(String, Direction, Rect)> = Vec::new();
    let mut drag_hotspots: Vec<(String, Direction, String, Rect)> = Vec::new();

    // --- Геометрия: фиксированные пропорции тела объекта ---
    let req_count = object.requests.len();
    let out_count = object.outputs.len();
    let total_h = rect_px.height();
    let header_h_px = total_h * 0.10;
    let pad = 8.0 * zoom;
    let container_rect = Rect::from_min_size(
        Pos2::new(rect_px.min.x + pad, rect_px.min.y + header_h_px + pad),
        Vec2::new(rect_px.width() - 2.0 * pad, total_h - header_h_px - 2.0 * pad),
    );
    let container_h = container_rect.height();
    let gap = pad;
    let split_h = container_h - gap;
    let req_h_px = split_h * 0.5;
    let out_h_px = split_h * 0.5;

    // --- Внешняя рамка объекта ---
    let border_color = if is_hovered { theme.text } else { theme.card_border };
    let border_width = if is_hovered {
        theme.object_border_hover_width * zoom
    } else {
        theme.object_border_width * zoom
    };
    painter.rect_filled(rect_px, 0.0, theme.card_bg);
    painter.rect_stroke(
        rect_px,
        0.0,
        Stroke::new(border_width, border_color),
    );

    // --- Заголовок объекта (имя + #ID) ---
    let header_rect = Rect::from_min_size(rect_px.min, Vec2::new(rect_px.width(), header_h_px));
    painter.rect_filled(header_rect, 0.0, theme.card_bg);

    let title = format!("{} '{}'", object.kind.label_ru(), object.name);
    let name_color = if is_hovered { theme.text } else { theme.text_muted };
    painter.text(
        Pos2::new(rect_px.min.x + 8.0 * zoom, rect_px.min.y + header_h_px / 2.0),
        egui::Align2::LEFT_CENTER,
        &title,
        egui::FontId::new(theme.text_size * zoom, egui::FontFamily::Proportional),
        name_color,
    );

    let id_text = format!("#{}", object.id);
    painter.text(
        Pos2::new(rect_px.max.x - 8.0 * zoom, rect_px.min.y + header_h_px / 2.0),
        egui::Align2::RIGHT_CENTER,
        &id_text,
        egui::FontId::new(theme.text_size * zoom, egui::FontFamily::Proportional),
        name_color,
    );

    // =============== КОНТЕЙНЕР: ЗАПРОС ===============
    let req_rect = Rect::from_min_size(
        container_rect.min,
        Vec2::new(container_rect.width(), req_h_px),
    );
    // Граница контейнера
    let req_border = if is_hovered { theme.text } else { theme.card_border };
    painter.rect_stroke(
        req_rect,
        0.0,
        Stroke::new(theme.object_border_width * zoom, req_border),
    );

    // Шапка контейнера + кнопка меню
    let req_header_h = req_h_px * 0.20;
    let req_header_rect = Rect::from_min_size(req_rect.min, Vec2::new(req_rect.width(), req_header_h));
    painter.rect_filled(req_header_rect, 0.0, theme.card_bg);
    if let Some(btn_rect) = paint_container_header(
        painter, req_header_rect, Direction::Request, &object.requests,
        dynamic, zoom, theme, is_hovered,
    ) {
        menu_hotspots.push((object.id.clone(), Direction::Request, btn_rect));
    }

    // Строки показателей + строка добавления
    let content_top = req_rect.min.y + req_header_h;
    let content_h = req_rect.height() - req_header_h;
    let visible_req = req_count.min(MAX_VISIBLE_METRICS);
    let show_add_req = visible_req < MAX_VISIBLE_METRICS;
    let total_rows_req = visible_req + if show_add_req { 1 } else { 0 };
    let gap_total_req = (MAX_VISIBLE_METRICS - 1) as f32 * ROW_GAP_PX * zoom;
    let mut row_h_req = if MAX_VISIBLE_METRICS > 0 {
        (content_h - gap_total_req) / MAX_VISIBLE_METRICS as f32
    } else {
        0.0
    };
    row_h_req = row_h_req.max(MIN_ROW_H_PX * zoom).floor();
    let row_step_req = row_h_req + ROW_GAP_PX * zoom;

    for (i, entry) in object.requests.iter().enumerate().take(MAX_VISIBLE_METRICS) {
        let row_rect = Rect::from_min_size(
            Pos2::new(req_rect.min.x, content_top + i as f32 * row_step_req),
            Vec2::new(req_rect.width(), row_h_req),
        );
        if is_hovered {
            painter.rect_filled(row_rect, 0.0, theme.card_bg);
        }
        let binding_key = (object.id.clone(), entry.name.clone());
        let binding = bindings.get(&binding_key);
        paint_metric_row(painter, row_rect, entry, dynamic, zoom, theme, is_hovered, Direction::Request, binding, error_hotspots);
        drag_hotspots.push((
            object.id.clone(),
            Direction::Request,
            entry.name.clone(),
            row_rect,
        ));
    }
    if show_add_req {
        let add_rect = Rect::from_min_size(
            Pos2::new(req_rect.min.x, content_top + visible_req as f32 * row_step_req),
            Vec2::new(req_rect.width(), row_h_req),
        );
        paint_add_row(painter, add_rect, zoom, theme, is_hovered);
        add_hotspots.push((object.id.clone(), Direction::Request, add_rect));
    }

    // =============== КОНТЕЙНЕР: ВЫХОД ===============
    let out_rect = Rect::from_min_size(
        Pos2::new(container_rect.min.x, container_rect.min.y + req_h_px + gap),
        Vec2::new(container_rect.width(), out_h_px),
    );
    let out_border = if is_hovered { theme.text } else { theme.card_border };
    painter.rect_stroke(
        out_rect,
        0.0,
        Stroke::new(theme.object_border_width * zoom, out_border),
    );

    // Шапка контейнера + кнопка меню
    let out_header_h = out_h_px * 0.20;
    let out_header_rect = Rect::from_min_size(out_rect.min, Vec2::new(out_rect.width(), out_header_h));
    painter.rect_filled(out_header_rect, 0.0, theme.card_bg);
    if let Some(btn_rect) = paint_container_header(
        painter, out_header_rect, Direction::Output, &object.outputs,
        dynamic, zoom, theme, is_hovered,
    ) {
        menu_hotspots.push((object.id.clone(), Direction::Output, btn_rect));
    }

    let content_top_out = out_rect.min.y + out_header_h;
    let content_h_out = out_rect.height() - out_header_h;
    let visible_out = out_count.min(MAX_VISIBLE_METRICS);
    let show_add_out = visible_out < MAX_VISIBLE_METRICS;
    let total_rows_out = visible_out + if show_add_out { 1 } else { 0 };
    let gap_total_out = (MAX_VISIBLE_METRICS - 1) as f32 * ROW_GAP_PX * zoom;
    let mut row_h_out = if MAX_VISIBLE_METRICS > 0 {
        (content_h_out - gap_total_out) / MAX_VISIBLE_METRICS as f32
    } else {
        0.0
    };
    row_h_out = row_h_out.max(MIN_ROW_H_PX * zoom).floor();
    let row_step_out = row_h_out + ROW_GAP_PX * zoom;

    for (i, entry) in object.outputs.iter().enumerate().take(MAX_VISIBLE_METRICS) {
        let row_rect = Rect::from_min_size(
            Pos2::new(out_rect.min.x, content_top_out + i as f32 * row_step_out),
            Vec2::new(out_rect.width(), row_h_out),
        );
        if is_hovered {
            painter.rect_filled(row_rect, 0.0, theme.card_bg);
        }
        let binding_key = (object.id.clone(), entry.name.clone());
        let binding = bindings.get(&binding_key);
        paint_metric_row(painter, row_rect, entry, dynamic, zoom, theme, is_hovered, Direction::Output, binding, error_hotspots);
        drag_hotspots.push((
            object.id.clone(),
            Direction::Output,
            entry.name.clone(),
            row_rect,
        ));
    }
    if show_add_out {
        let add_rect = Rect::from_min_size(
            Pos2::new(out_rect.min.x, content_top_out + visible_out as f32 * row_step_out),
            Vec2::new(out_rect.width(), row_h_out),
        );
        paint_add_row(painter, add_rect, zoom, theme, is_hovered);
        add_hotspots.push((object.id.clone(), Direction::Output, add_rect));
    }

    (add_hotspots, menu_hotspots, drag_hotspots)
}

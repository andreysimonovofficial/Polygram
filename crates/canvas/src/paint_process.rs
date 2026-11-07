//! Процедурный рендер объекта "Процесс" (конструктор-style, без design.json).
//!
//! Все константы в начале файла — крути их, результат меняется сразу.
//! Высота объекта растёт под контент: заголовок + 2 × (заголовок контейнера + строки показателей).
//! Всё считается из констант: нет наложения статики и динамики, нет двойной отрисовки.

use crate::theme::Theme;
use egui::{Painter, Pos2, Rect, Stroke, Vec2};
use polygram_core::types::Direction;
use polygram_core::{DynamicState, ObjectGeom, StatusCode};

// ┌─────────────────────────────────────────────────────────────┐
// │ КОНСТАНТЫ-КОНСТРУКТОР                                       │
// └─────────────────────────────────────────────────────────────┘

/// Максимум видимых показателей на сторону (лимит UI, не ядра).
pub const MAX_VISIBLE_METRICS: usize = 4;

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

/// Рисовать одну строку показателя: точка статуса слева, имя, значение справа.
fn paint_metric_row(
    painter: &Painter,
    rect: Rect,
    entry: &polygram_core::EntryGeom,
    dynamic: &DynamicState,
    zoom: f32,
    theme: &Theme,
    is_hovered: bool,
) {
    let status = dynamic.status[entry.ix as usize];
    let dot_color = super::status_dot_color(status, theme);
    let dot_radius = theme.port_radius.max(2.0) * zoom;
    let cy = rect.center().y;

    // Точка статуса
    painter.circle_filled(
        Pos2::new(rect.min.x + 8.0 * zoom, cy),
        dot_radius,
        dot_color,
    );

    // Имя записи
    let name_color = if is_hovered { theme.text } else { theme.text_muted };
    painter.text(
        Pos2::new(rect.min.x + 16.0 * zoom, cy),
        egui::Align2::LEFT_CENTER,
        &entry.name,
        egui::FontId::new(theme.text_size * zoom, egui::FontFamily::Proportional),
        name_color,
    );

    // Значение (справа)
    let value_text = format!("{:.0}", entry.value);
    painter.text(
        Pos2::new(rect.max.x - 8.0 * zoom, cy),
        egui::Align2::RIGHT_CENTER,
        &value_text,
        egui::FontId::new(theme.text_size * zoom, egui::FontFamily::Proportional),
        name_color,
    );
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
fn paint_container_header(
    painter: &Painter,
    rect: Rect,
    dir: Direction,
    entries: &[polygram_core::EntryGeom],
    dynamic: &DynamicState,
    zoom: f32,
    theme: &Theme,
    is_hovered: bool,
) {
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
        super::status_dot_color(status, theme)
    } else {
        theme.text_muted
    };
    let text_color = if is_hovered { theme.text } else { header_color };

    // Точка статуса контейнера (слева)
    let dot_color = super::status_dot_color(status, theme);
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
}

/// Высокопроизводительный процедурный рендер объекта "Процесс".
/// Рисует: внешняя рамка → заголовок объекта → контейнер Запрос → контейнер Выход.
/// Возвращает горячие зоны добавления показателя (для клика «+ Добавить показатель»).
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
) -> Vec<(String, Direction, Rect)> {
    let mut hotspots: Vec<(String, Direction, Rect)> = Vec::new();

    // --- Геометрия: фиксированные пропорции тела объекта ---
    let req_count = object.requests.len();
    let out_count = object.outputs.len();
    let total_h = rect_px.height();
    let header_h_px = total_h * 0.10;
    let pad = 8.0 * zoom;
    let container_rect = Rect::from_min_size(
        Pos2::new(rect_px.min.x + pad, rect_px.min.y + header_h_px),
        Vec2::new(rect_px.width() - 2.0 * pad, total_h - header_h_px - pad),
    );
    let container_h = container_rect.height();
    let req_h_px = container_h * 0.5;
    let out_h_px = container_h * 0.5;

    // --- Внешняя рамка объекта ---
    let border_color = if is_hovered { theme.text } else { theme.card_border };
    let border_width = if is_hovered {
        theme.object_border_hover_width * zoom
    } else {
        theme.object_border_width * zoom
    };
    painter.rect_filled(rect_px, theme.object_corner_radius * zoom, theme.card_bg);
    painter.rect_stroke(
        rect_px,
        theme.object_corner_radius * zoom,
        Stroke::new(border_width, border_color),
    );

    // --- Заголовок объекта (имя + #ID) ---
    let header_rect = Rect::from_min_size(rect_px.min, Vec2::new(rect_px.width(), header_h_px));
    painter.rect_filled(header_rect, 0.0, theme.card_header);

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
        theme.object_corner_radius * zoom * 0.5,
        Stroke::new(theme.object_border_width * zoom, req_border),
    );

    // Шапка контейнера
    let req_header_h = req_h_px * 0.20;
    let req_header_rect = Rect::from_min_size(req_rect.min, Vec2::new(req_rect.width(), req_header_h));
    painter.rect_filled(req_header_rect, 0.0, theme.card_bg);
    paint_container_header(
        painter, req_header_rect, Direction::Request, &object.requests,
        dynamic, zoom, theme, is_hovered,
    );

    // Строки показателей + строка добавления
    let content_top = req_rect.min.y + req_header_h;
    let content_h = req_rect.height() - req_header_h;
    let visible_req = req_count.min(MAX_VISIBLE_METRICS);
    let show_add_req = visible_req < MAX_VISIBLE_METRICS;
    let total_rows_req = visible_req + if show_add_req { 1 } else { 0 };
    let row_h_req = if total_rows_req > 0 { content_h / total_rows_req as f32 } else { 0.0 };

    for (i, entry) in object.requests.iter().enumerate().take(MAX_VISIBLE_METRICS) {
        let row_rect = Rect::from_min_size(
            Pos2::new(req_rect.min.x, content_top + i as f32 * row_h_req),
            Vec2::new(req_rect.width(), row_h_req),
        );
        if is_hovered {
            painter.rect_filled(row_rect, 0.0, theme.card_header);
        }
        paint_metric_row(painter, row_rect, entry, dynamic, zoom, theme, is_hovered);
    }
    if show_add_req {
        let add_rect = Rect::from_min_size(
            Pos2::new(req_rect.min.x, content_top + visible_req as f32 * row_h_req),
            Vec2::new(req_rect.width(), row_h_req),
        );
        paint_add_row(painter, add_rect, zoom, theme, is_hovered);
        hotspots.push((object.id.clone(), Direction::Request, add_rect));
    }

    // =============== КОНТЕЙНЕР: ВЫХОД ===============
    let out_rect = Rect::from_min_size(
        Pos2::new(container_rect.min.x, container_rect.min.y + req_h_px),
        Vec2::new(container_rect.width(), out_h_px),
    );
    let out_border = if is_hovered { theme.text } else { theme.card_border };
    painter.rect_stroke(
        out_rect,
        theme.object_corner_radius * zoom * 0.5,
        Stroke::new(theme.object_border_width * zoom, out_border),
    );

    let out_header_h = out_h_px * 0.20;
    let out_header_rect = Rect::from_min_size(out_rect.min, Vec2::new(out_rect.width(), out_header_h));
    painter.rect_filled(out_header_rect, 0.0, theme.card_bg);
    paint_container_header(
        painter, out_header_rect, Direction::Output, &object.outputs,
        dynamic, zoom, theme, is_hovered,
    );

    let content_top_out = out_rect.min.y + out_header_h;
    let content_h_out = out_rect.height() - out_header_h;
    let visible_out = out_count.min(MAX_VISIBLE_METRICS);
    let show_add_out = visible_out < MAX_VISIBLE_METRICS;
    let total_rows_out = visible_out + if show_add_out { 1 } else { 0 };
    let row_h_out = if total_rows_out > 0 { content_h_out / total_rows_out as f32 } else { 0.0 };

    for (i, entry) in object.outputs.iter().enumerate().take(MAX_VISIBLE_METRICS) {
        let row_rect = Rect::from_min_size(
            Pos2::new(out_rect.min.x, content_top_out + i as f32 * row_h_out),
            Vec2::new(out_rect.width(), row_h_out),
        );
        if is_hovered {
            painter.rect_filled(row_rect, 0.0, theme.card_header);
        }
        paint_metric_row(painter, row_rect, entry, dynamic, zoom, theme, is_hovered);
    }
    if show_add_out {
        let add_rect = Rect::from_min_size(
            Pos2::new(out_rect.min.x, content_top_out + visible_out as f32 * row_h_out),
            Vec2::new(out_rect.width(), row_h_out),
        );
        paint_add_row(painter, add_rect, zoom, theme, is_hovered);
        hotspots.push((object.id.clone(), Direction::Output, add_rect));
    }

    hotspots
}

//! Рисование объекта "Процесс" и пустой пунктирной рамки ("призрачной копии").

use egui::{Painter, Pos2, Rect, Stroke, Vec2};
use polygram_core::types::Direction;
use polygram_core::{DynamicState, EntryGeom, ObjectGeom, StatusCode};
use std::collections::HashMap;

use crate::paint::constants::MAX_VISIBLE_METRICS;
use crate::paint::status::worst_status;
use crate::status_dot_color;
use crate::status_label_ru;

/// Высокопроизводительный процедурный рендер объекта "Процесс" по фиксированной CSS-геометрии.
/// Карточка 256×256: заголовок (Имя + #ID + ◆) → контейнер Запрос (y:32) → контейнер Выход (y:144).
/// Каждый контейнер 248×108: шапка (LABEL (N) СТАТУС + ◆) + 4 строки показателя на фиксированных y (28, 48, 68, 88).
/// Возвращает (горячие зоны добавления показателя, горячие зоны меню контейнеров, горячие зоны перетаскивания показателей).
pub fn paint_process(
    painter: &Painter,
    object: &ObjectGeom,
    dynamic: &DynamicState,
    theme: &crate::theme::Theme,
    rect_px: Rect,
    _cell_px: f32,
    zoom: f32,
    is_hovered: bool,
    _edit_target: Option<&str>,
    bindings: &HashMap<(String, String), crate::paint::row::MetricBinding>,
    error_hotspots: &mut Vec<(Rect, String)>,
) -> (
    Vec<(String, Direction, Rect)>,
    Vec<(String, Direction, Rect)>,
    Vec<(String, Direction, String, Rect)>,
) {
    let mut add_hotspots: Vec<(String, Direction, Rect)> = Vec::new();
    let mut menu_hotspots: Vec<(String, Direction, Rect)> = Vec::new();
    let mut drag_hotspots: Vec<(String, Direction, String, Rect)> = Vec::new();

    let ox = rect_px.min.x;
    let oy = rect_px.min.y;

    // --- Внешняя рамка объекта (256×256) ---
    // CSS spec defines fixed card size 256×256px regardless of stored cell dimensions.
    let card_w = 256.0 * zoom;
    let card_h = 256.0 * zoom;
    let card_rect = Rect::from_min_size(Pos2::new(ox, oy), Vec2::new(card_w, card_h));
    let border_color = if is_hovered { theme.text } else { theme.card_border };
    let border_width = if is_hovered {
        theme.object_border_hover_width * zoom
    } else {
        theme.object_border_width * zoom
    };
    painter.rect_filled(card_rect, 0.0, theme.card_bg);
    painter.rect_stroke(
        card_rect,
        0.0,
        Stroke::new(border_width, border_color),
        egui::StrokeKind::Middle,
    );

    // --- Заголовок объекта: ◆ + {Имя процесса} + {ID процесса} ---
    // CSS: ◆ left:8 top:4 h:28 → center y=18; Имя left:24 top:4 h:28 → center y=18; #ID left:8 top:4 h:28 → center y=18
    let header_h = 28.0 * zoom;
    let header_cy = oy + 4.0 * zoom + header_h / 2.0;
    painter.text(
        Pos2::new(ox + 16.0 * zoom, header_cy),
        egui::Align2::CENTER_CENTER,
        "◆",
        egui::FontId::new(10.0 * zoom, egui::FontFamily::Proportional),
        theme.text,
    );
    let id_text = format!("#{}", object.id);
    painter.text(
        Pos2::new(ox + 248.0 * zoom, header_cy),
        egui::Align2::RIGHT_CENTER,
        &id_text,
        egui::FontId::new(10.0 * zoom, egui::FontFamily::Proportional),
        theme.text,
    );
    let title = format!("{} '{}'", object.kind.label_ru(), object.name);
    painter.text(
        Pos2::new(ox + 24.0 * zoom, header_cy),
        egui::Align2::LEFT_CENTER,
        &title,
        egui::FontId::new(10.0 * zoom, egui::FontFamily::Proportional),
        theme.text,
    );

    // Геометрия контейнеров (из CSS-спецификации)
    let container_w = 248.0 * zoom;
    let container_h = 108.0 * zoom;
    let container_x = ox + 4.0 * zoom;
    let row_w = 240.0 * zoom;
    let row_h = 16.0 * zoom;
    let container_header_h = 24.0 * zoom;
    // Смещения строк показателя внутри контейнера: 28, 48, 68, 88
    let row_offsets: [f32; 4] = [28.0, 48.0, 68.0, 88.0];

    // === Контейнер: ЗАПРОС (top: 32) ===
    let req_rect = Rect::from_min_size(
        Pos2::new(container_x, oy + 32.0 * zoom),
        Vec2::new(container_w, container_h),
    );
    paint_metric_container(
        painter, object, dynamic, theme, zoom, is_hovered,
        Direction::Request, &object.requests, req_rect,
        container_header_h, row_w, row_h, container_x, &row_offsets,
        bindings, error_hotspots,
        &mut add_hotspots, &mut menu_hotspots, &mut drag_hotspots,
    );

    // === Контейнер: ВЫХОД (top: 144) ===
    let out_rect = Rect::from_min_size(
        Pos2::new(container_x, oy + 144.0 * zoom),
        Vec2::new(container_w, container_h),
    );
    paint_metric_container(
        painter, object, dynamic, theme, zoom, is_hovered,
        Direction::Output, &object.outputs, out_rect,
        container_header_h, row_w, row_h, container_x, &row_offsets,
        bindings, error_hotspots,
        &mut add_hotspots, &mut menu_hotspots, &mut drag_hotspots,
    );

    (add_hotspots, menu_hotspots, drag_hotspots)
}

/// Рисование одного контейнера показателей (Запрос или Выход): рамка, шапка, строки.
fn paint_metric_container(
    painter: &Painter,
    object: &ObjectGeom,
    dynamic: &DynamicState,
    theme: &crate::theme::Theme,
    zoom: f32,
    is_hovered: bool,
    dir: Direction,
    entries: &[EntryGeom],
    container_rect: Rect,
    container_header_h: f32,
    row_w: f32,
    row_h: f32,
    _container_x: f32,
    row_offsets: &[f32; 4],
    _bindings: &HashMap<(String, String), crate::paint::row::MetricBinding>,
    _error_hotspots: &mut Vec<(Rect, String)>,
    add_hotspots: &mut Vec<(String, Direction, Rect)>,
    menu_hotspots: &mut Vec<(String, Direction, Rect)>,
    drag_hotspots: &mut Vec<(String, Direction, String, Rect)>,
) {
    let cx = container_rect.min.x;
    let cy = container_rect.min.y;

    // --- Рамка контейнера ---
    let border_color = if is_hovered { theme.text } else { theme.card_border };
    painter.rect_stroke(
        container_rect,
        0.0,
        Stroke::new(theme.object_border_width * zoom, border_color),
        egui::StrokeKind::Middle,
    );

    // --- Шапка контейнера: ◆ слева (top:4), текст (left:20, top:4), высота 24 ---
    let header_rect = Rect::from_min_size(
        Pos2::new(cx + 0.0 * zoom, cy + 4.0 * zoom),
        Vec2::new(container_rect.width(), container_header_h),
    );
    painter.rect_filled(header_rect, 0.0, theme.card_bg);

    // Статус контейнера
    let status = worst_status(entries, dynamic);
    let dot_color = status_dot_color(status, dir, theme);
    let label = match dir {
        Direction::Request => "ЗАПРОС",
        Direction::Output => "ВЫХОД",
    };
    let status_text = status_label_ru(status, dir);
    let problem_count = entries
        .iter()
        .filter(|e| dynamic.status[e.ix as usize] != StatusCode::Ok)
        .count();
    let status_full = if problem_count > 0 {
        format!("{} ({})", status_text, problem_count)
    } else {
        status_text.to_string()
    };
    let header_text_color = if is_hovered { theme.text } else { dot_color };
    let header_cy = cy + 16.0 * zoom; // top:4 + h:24/2 = 16

    // ◆ символ (слева, left:4, top:4, center x=12 y=16)
    painter.text(
        Pos2::new(cx + 12.0 * zoom, header_cy),
        egui::Align2::CENTER_CENTER,
        "◆",
        egui::FontId::new(9.0 * zoom, egui::FontFamily::Proportional),
        theme.text,
    );

    // Текст шапки: "ЗАПРОС (N) СТАТУС" (left:20, top:4, center y=16)
    let count_text = format!("{} ({}) {}", label, entries.len(), status_full);
    painter.text(
        Pos2::new(cx + 20.0 * zoom, header_cy),
        egui::Align2::LEFT_CENTER,
        &count_text,
        egui::FontId::new(9.0 * zoom, egui::FontFamily::Proportional),
        header_text_color,
    );

    // Горячая зона меню (область ◆: left:4, top:4, w:16, h:24 относительно контейнера)
    let menu_rect = Rect::from_min_size(
        Pos2::new(cx + 4.0 * zoom, cy + 4.0 * zoom),
        Vec2::new(16.0 * zoom, container_header_h),
    );
    menu_hotspots.push((object.id.clone(), dir, menu_rect));

    // --- Строки показателя (top: 28, 48, 68, 88 относительно контейнера) ---
    let visible = entries.len().min(MAX_VISIBLE_METRICS);
    let show_add = visible < MAX_VISIBLE_METRICS;

    let (left_sym, right_sym) = match dir {
        Direction::Request => ("◀", "▶"),
        Direction::Output => ("▶", "◀"),
    };
    let symbol_font = egui::FontId::new((8.0 * zoom).max(10.0 * zoom), egui::FontFamily::Proportional);

    for (i, entry) in entries.iter().enumerate().take(MAX_VISIBLE_METRICS) {
        let row_y = cy + row_offsets[i] * zoom;
        let row_x = cx + 4.0 * zoom; // left:4 относительно контейнера
        let row_rect = Rect::from_min_size(Pos2::new(row_x, row_y), Vec2::new(row_w, row_h));
        if is_hovered {
            painter.rect_filled(row_rect, 0.0, theme.card_bg);
        }
        // Рамка строки
        painter.rect_stroke(
            row_rect,
            0.0,
            Stroke::new(theme.object_border_width * zoom, theme.card_border),
            egui::StrokeKind::Middle,
        );
        let row_status = dynamic.status[entry.ix as usize];
        let row_dot_color = status_dot_color(row_status, dir, theme);
        let cy_row = row_rect.center().y;

        // Символ сцепления слева (left: 0 относительно строки → cx+4)
        painter.text(
            Pos2::new(row_x, cy_row),
            egui::Align2::LEFT_CENTER,
            left_sym,
            symbol_font.clone(),
            row_dot_color,
        );
        // Символ сцепления справа (left: 224 относительно строки → cx+4+224)
        painter.text(
            Pos2::new(row_x + 224.0 * zoom, cy_row),
            egui::Align2::LEFT_CENTER,
            right_sym,
            symbol_font.clone(),
            row_dot_color,
        );
        // Имя (left: 16 относительно строки → cx+4+16)
        let name_color = if is_hovered { theme.text } else { theme.text_muted };
        painter.text(
            Pos2::new(row_x + 16.0 * zoom, cy_row),
            egui::Align2::LEFT_CENTER,
            &entry.name,
            egui::FontId::new(8.0 * zoom, egui::FontFamily::Proportional),
            name_color,
        );
        // Значение + ед.измерения (right-align, заканчивается в 224 относительно строки → cx+4+224)
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
            Pos2::new(row_x + 224.0 * zoom, cy_row),
            egui::Align2::RIGHT_CENTER,
            &value_text,
            egui::FontId::new(8.0 * zoom, egui::FontFamily::Proportional),
            name_color,
        );

        // Горячая зона перетаскивания строки
        drag_hotspots.push((object.id.clone(), dir, entry.name.clone(), row_rect));
    }

    // Строка "+ Добавить показатель" (если есть место)
    if show_add {
        let add_y = cy + row_offsets[visible] * zoom;
        let add_rect = Rect::from_min_size(Pos2::new(cx + 4.0 * zoom, add_y), Vec2::new(row_w, row_h));
        let add_text = "+ Добавить показатель";
        let add_color = if is_hovered { theme.text } else { theme.text_muted };
        painter.text(
            add_rect.center(),
            egui::Align2::CENTER_CENTER,
            add_text,
            egui::FontId::new(8.0 * zoom, egui::FontFamily::Proportional),
            add_color,
        );
        add_hotspots.push((object.id.clone(), dir, add_rect));
    }
}

/// Пустая пунктирная рамка ("призрачная копия") размером с объект.
/// Рисуется только обводка пунктиром без заливки и содержимого.
pub fn paint_dashed_rect(painter: &Painter, rect: Rect, zoom: f32, color: egui::Color32) {
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

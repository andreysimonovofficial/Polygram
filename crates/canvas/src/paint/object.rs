//! Рисование объекта "Процесс" и пустой пунктирной рамки ("призрачной копии").

use egui::{Painter, Pos2, Rect, Stroke, Vec2};
use polygram_core::types::Direction;
use polygram_core::{DynamicState, EntryGeom, ObjectGeom, StatusCode};
use std::collections::HashMap;

use crate::paint::constants::{MAX_VISIBLE_METRICS, SYMBOL_X_PAD, TEXT_X_PAD};
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
    mouse_pos: Option<Pos2>,
    _edit_target: Option<&str>,
    is_selected: bool,
    bindings: &HashMap<(String, String), crate::paint::row::MetricBinding>,
    error_hotspots: &mut Vec<(Rect, String)>,
) -> (
    Vec<(String, Direction, Rect)>,
    Vec<(String, Direction, Rect)>,
    Vec<(String, Direction, String, Rect)>,
    Vec<(String, Direction, Rect)>,
) {
    let mut add_hotspots: Vec<(String, Direction, Rect)> = Vec::new();
    let mut menu_hotspots: Vec<(String, Direction, Rect)> = Vec::new();
    let mut header_hotspots: Vec<(String, Direction, Rect)> = Vec::new();
    let mut drag_hotspots: Vec<(String, Direction, String, Rect)> = Vec::new();

    let ox = rect_px.min.x;
    let oy = rect_px.min.y;

    // --- Внешняя рамка объекта (256×256) ---
    // CSS spec defines fixed card size 256×256px regardless of stored cell dimensions.
    let card_w = 256.0 * zoom;
    let card_h = 256.0 * zoom;
    let card_rect = Rect::from_min_size(Pos2::new(ox, oy), Vec2::new(card_w, card_h));
    painter.rect_filled(card_rect, 0.0, theme.card_bg);

    // Геометрия контейнеров (из CSS-спецификации)
    let container_w = 248.0 * zoom;
    let container_h = 108.0 * zoom;
    let container_x = ox + 4.0 * zoom;
    let row_w = 240.0 * zoom;
    let row_h = 16.0 * zoom;
    let container_header_h = 24.0 * zoom;
    // Смещения строк показателя внутри контейнера: 28, 48, 68, 88
    let row_offsets: [f32; 4] = [28.0, 48.0, 68.0, 88.0];

    // Определяем зону наведения (взаимоисключающе):
    // строка показателя > контейнер > тело карточки.
    let mut container_hovered_dir: Option<Direction> = None;
    let mut row_hovered_rect: Option<Rect> = None;

    if let Some(pos) = mouse_pos {
        // Сначала проверяем строки показателя (самое конкретное)
        for (dir, entries) in [
            (Direction::Request, &object.requests as &[EntryGeom]),
            (Direction::Output, &object.outputs as &[EntryGeom]),
        ] {
            let container_top = if dir == Direction::Request {
                oy + 32.0 * zoom
            } else {
                oy + 144.0 * zoom
            };
            let visible = entries.len().min(MAX_VISIBLE_METRICS);
            for i in 0..visible {
                let row_y = container_top + row_offsets[i] * zoom;
                let row_x = container_x + 4.0 * zoom;
                // Прямоугольник для hit-testing строки расширен вниз до следующей строки
                // (или до низа контейнера для последней), чтобы при движении мыши по
                // промежуткам между строками не вспыхивал контейнер Запрос/Выход.
                let hit_bottom = if i + 1 < visible {
                    container_top + row_offsets[i + 1] * zoom
                } else {
                    container_top + container_h
                };
                let hit_rect = Rect::from_min_size(Pos2::new(row_x, row_y), Vec2::new(row_w, hit_bottom - row_y));
                if hit_rect.contains(pos) {
                    row_hovered_rect = Some(hit_rect);
                    break;
                }
            }
            if row_hovered_rect.is_some() {
                break;
            }
        }

        // Затем проверяем контейнеры (только если не над строкой)
        if row_hovered_rect.is_none() {
            let req_rect = Rect::from_min_size(
                Pos2::new(container_x, oy + 32.0 * zoom),
                Vec2::new(container_w, container_h),
            );
            let out_rect = Rect::from_min_size(
                Pos2::new(container_x, oy + 144.0 * zoom),
                Vec2::new(container_w, container_h),
            );
            if req_rect.contains(pos) {
                container_hovered_dir = Some(Direction::Request);
            } else if out_rect.contains(pos) {
                container_hovered_dir = Some(Direction::Output);
            }
        }
    }

    let body_hovered = mouse_pos.map_or(false, |pos| card_rect.contains(pos))
        && container_hovered_dir.is_none()
        && row_hovered_rect.is_none();
    let body_highlighted = body_hovered || is_selected;

    let border_color = if body_highlighted { theme.text } else { theme.card_border };
    let border_width = if body_highlighted {
        theme.object_border_hover_width * zoom
    } else {
        theme.object_border_width * zoom
    };
    painter.rect_stroke(
        card_rect,
        0.0,
        Stroke::new(border_width, border_color),
        egui::StrokeKind::Middle,
    );

    let body_text_color = if body_highlighted { theme.text } else { theme.text_muted };

    // --- Заголовок объекта: ◆ + {Имя процесса} + {ID процесса} ---
    // CSS: ◆ left:8 top:4 h:28 → center y=18; Имя left:24 top:4 h:28 → center y=18; #ID left:8 top:4 h:28 → center y=18
    let header_h = 28.0 * zoom;
    let header_cy = oy + 4.0 * zoom + header_h / 2.0;
    painter.text(
        Pos2::new(ox + 16.0 * zoom, header_cy),
        egui::Align2::CENTER_CENTER,
        "◆",
        egui::FontId::new(10.0 * zoom, egui::FontFamily::Proportional),
        body_text_color,
    );
    let id_text = format!("#{}", object.id);
    painter.text(
        Pos2::new(ox + 248.0 * zoom, header_cy),
        egui::Align2::RIGHT_CENTER,
        &id_text,
        egui::FontId::new(10.0 * zoom, egui::FontFamily::Proportional),
        body_text_color,
    );
    let title = format!("{} '{}'", object.kind.label_ru(), object.name);
    painter.text(
        Pos2::new(ox + 24.0 * zoom, header_cy),
        egui::Align2::LEFT_CENTER,
        &title,
        egui::FontId::new(10.0 * zoom, egui::FontFamily::Proportional),
        body_text_color,
    );

    // === Контейнер: ЗАПРОС (top: 32) ===
    let req_rect = Rect::from_min_size(
        Pos2::new(container_x, oy + 32.0 * zoom),
        Vec2::new(container_w, container_h),
    );
    let req_hovered = container_hovered_dir == Some(Direction::Request);
    paint_metric_container(
        painter, object, dynamic, theme, zoom,
        req_hovered,
        mouse_pos,
        Direction::Request, &object.requests, req_rect,
        container_header_h, row_w, row_h, container_x, &row_offsets,
        bindings, error_hotspots,
        &mut add_hotspots, &mut menu_hotspots, &mut header_hotspots, &mut drag_hotspots,
    );

    // === Контейнер: ВЫХОД (top: 144) ===
    let out_rect = Rect::from_min_size(
        Pos2::new(container_x, oy + 144.0 * zoom),
        Vec2::new(container_w, container_h),
    );
    let out_hovered = container_hovered_dir == Some(Direction::Output);
    paint_metric_container(
        painter, object, dynamic, theme, zoom,
        out_hovered,
        mouse_pos,
        Direction::Output, &object.outputs, out_rect,
        container_header_h, row_w, row_h, container_x, &row_offsets,
        bindings, error_hotspots,
        &mut add_hotspots, &mut menu_hotspots, &mut header_hotspots, &mut drag_hotspots,
    );

    (add_hotspots, menu_hotspots, drag_hotspots, header_hotspots)
}

/// Рисование одного контейнера показателей (Запрос или Выход): рамка, шапка, строки.
fn paint_metric_container(
    painter: &Painter,
    object: &ObjectGeom,
    dynamic: &DynamicState,
    theme: &crate::theme::Theme,
    zoom: f32,
    container_hovered: bool,
    mouse_pos: Option<Pos2>,
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
    header_hotspots: &mut Vec<(String, Direction, Rect)>,
    drag_hotspots: &mut Vec<(String, Direction, String, Rect)>,
) {
    let cx = container_rect.min.x;
    let cy = container_rect.min.y;

    // --- Шапка контейнера: ◆ слева (top:4), текст (left:20, top:4), высота 24 ---
    let header_rect = Rect::from_min_size(
        Pos2::new(cx + 0.0 * zoom, cy + 4.0 * zoom),
        Vec2::new(container_rect.width(), container_header_h),
    );
    painter.rect_filled(header_rect, 0.0, theme.card_bg);

    // --- Рамка контейнера (рисуется после заливки шапки, чтобы не перекрывалась fill'ом) ---
    let border_color = if container_hovered { theme.text } else { theme.card_border };
    painter.rect_stroke(
        container_rect,
        0.0,
        Stroke::new(theme.object_border_width * zoom, border_color),
        egui::StrokeKind::Middle,
    );

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
    let header_text_color = if container_hovered { theme.text } else { dot_color };
    let header_cy = cy + 16.0 * zoom; // top:4 + h:24/2 = 16

    // ◆ символ (слева, left:4, top:4, center x=12 y=16)
    let header_symbol_color = if container_hovered { theme.text } else { theme.text_muted };
    painter.text(
        Pos2::new(cx + 12.0 * zoom, header_cy),
        egui::Align2::CENTER_CENTER,
        "◆",
        egui::FontId::new(9.0 * zoom, egui::FontFamily::Proportional),
        header_symbol_color,
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
    header_hotspots.push((object.id.clone(), dir, header_rect));

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
        let row_hovered = mouse_pos.map_or(false, |pos| row_rect.contains(pos));
        if row_hovered {
            painter.rect_filled(row_rect, 0.0, theme.card_bg);
        }
        // Рамка строки
        let row_border_color = if row_hovered { theme.text } else { theme.card_border };
        painter.rect_stroke(
            row_rect,
            0.0,
            Stroke::new(theme.object_border_width * zoom, row_border_color),
            egui::StrokeKind::Middle,
        );
        let row_status = dynamic.status[entry.ix as usize];
        let row_dot_color = status_dot_color(row_status, dir, theme);
        let cy_row = row_rect.center().y;

        // Символ сцепления слева с отступом от левого края строки
        painter.text(
            Pos2::new(row_x + SYMBOL_X_PAD * zoom, cy_row),
            egui::Align2::LEFT_CENTER,
            left_sym,
            symbol_font.clone(),
            row_dot_color,
        );
        // Символ сцепления справа с отступом от правого края строки
        painter.text(
            Pos2::new(row_x + row_w - SYMBOL_X_PAD * zoom, cy_row),
            egui::Align2::RIGHT_CENTER,
            right_sym,
            symbol_font.clone(),
            row_dot_color,
        );
        // Имя с отступом от левого края строки
        let row_text_color = if row_hovered { theme.text } else { theme.text_muted };
        painter.text(
            Pos2::new(row_x + TEXT_X_PAD * zoom, cy_row),
            egui::Align2::LEFT_CENTER,
            &entry.name,
            egui::FontId::new(8.0 * zoom, egui::FontFamily::Proportional),
            row_text_color,
        );
        // Значение + ед.измерения (right-align, отступ от правого края строки)
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
            Pos2::new(row_x + row_w - TEXT_X_PAD * zoom, cy_row),
            egui::Align2::RIGHT_CENTER,
            &value_text,
            egui::FontId::new(8.0 * zoom, egui::FontFamily::Proportional),
            row_text_color,
        );

        // Горячая зона перетаскивания строки
        drag_hotspots.push((object.id.clone(), dir, entry.name.clone(), row_rect));
    }

    // Строка "+ Добавить показатель" (если есть место)
    if show_add {
        let add_y = cy + row_offsets[visible] * zoom;
        let add_rect = Rect::from_min_size(Pos2::new(cx + 4.0 * zoom, add_y), Vec2::new(row_w, row_h));
        let add_text = "+ Добавить показатель";
        let add_color = if container_hovered { theme.text } else { theme.text_muted };
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

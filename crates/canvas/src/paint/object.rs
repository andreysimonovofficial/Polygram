//! Рисование объекта "Процесс" и пустой пунктирной рамки ("призрачной копии").

use egui::{Color32, Painter, Pos2, Rect, Stroke, Vec2};
use polygram_core::types::Direction;
use polygram_core::{DynamicState, ObjectGeom};
use std::collections::HashMap;

use crate::paint::constants::{MAX_VISIBLE_METRICS, MIN_ROW_H_PX, ROW_GAP_PX};
use crate::paint::container::paint_container_header;
use crate::paint::row::{paint_add_row, paint_metric_row};

/// Высокопроизводительный процедурный рендер объекта "Процесс".
/// Рисует: внешняя рамка → заголовок объекта → контейнер Запрос → контейнер Выход.
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

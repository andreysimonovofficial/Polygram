//! Рисование объекта "Процесс" и пустой пунктирной рамки ("призрачной копии").

use egui::{Painter, Pos2, Rect, Stroke, Vec2};
use polygram_core::types::Direction;
use polygram_core::{DynamicState, EntryGeom, ObjectGeom, StatusCode};

use crate::icons::paint_lucide;
use crate::paint::constants::{MAX_VISIBLE_METRICS, TEXT_X_PAD};
use crate::paint::status::worst_status;
use crate::status_dot_color;
use crate::status_label_ru;
use egui_lucide::Lucide;/// Высокопроизводительный процедурный рендер объекта "Процесс" по фиксированной CSS-геометрии.
/// Карточка 512×280: заголовок (Имя + #ID + ◆) → контейнер Запрос → контейнер Результат.
/// Каждый контейнер 496×96: шапка (LABEL (N) СТАТУС + ◆) + строки показателя.
/// Возвращает горячие зоны добавления показателя, горячие зоны меню контейнеров, горячие зоны перетаскивания показателей.
pub fn paint_process(
    ui: &egui::Ui,
    painter: &Painter,
    object: &ObjectGeom,
    dynamic: &DynamicState,
    theme: &crate::Theme,
    rect_px: Rect,
    _cell_px: f32,
    zoom: f32,
    mouse_pos: Option<Pos2>,
    is_selected: bool,
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

    // Процесс 512×280 px (CSS спецификация).
    let card_w = 512.0 * zoom;
    let card_h = 280.0 * zoom;
    let card_rect = Rect::from_min_size(Pos2::new(ox, oy), Vec2::new(card_w, card_h));
    painter.rect_filled(card_rect, 0.0, theme.card_bg);

    // Контейнер 496×96 px (CSS спецификация).
    let container_w = 496.0 * zoom;
    let container_h = 96.0 * zoom;
    let container_x = ox + 8.0 * zoom;
    let row_w = 480.0 * zoom;
    let row_h = 32.0 * zoom;
    let container_header_h = 48.0 * zoom;
    let row_offsets: [f32; 4] = [20.0, 60.0, 100.0, 140.0];

    let mut container_hovered_dir: Option<Direction> = None;
    let mut row_hovered_rect: Option<Rect> = None;

    if let Some(pos) = mouse_pos {
        for (dir, entries) in [
            (Direction::Request, &object.requests as &[EntryGeom]),
            (Direction::Result, &object.results as &[EntryGeom]),
        ] {
            let container_top = if dir == Direction::Request {
                oy + 72.0 * zoom
            } else {
                oy + 176.0 * zoom
            };
            let visible = entries.len().min(MAX_VISIBLE_METRICS);
            for i in 0..visible {
                let row_y = container_top + row_offsets[i] * zoom;
                let row_x = container_x + 8.0 * zoom;
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

        if row_hovered_rect.is_none() {
            let req_rect = Rect::from_min_size(
                Pos2::new(container_x, oy + 72.0 * zoom),
                Vec2::new(container_w, container_h),
            );
            let result_rect = Rect::from_min_size(
                Pos2::new(container_x, oy + 176.0 * zoom),
                Vec2::new(container_w, container_h),
            );
            if req_rect.contains(pos) {
                container_hovered_dir = Some(Direction::Request);
            } else if result_rect.contains(pos) {
                container_hovered_dir = Some(Direction::Result);
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

    let header_h = 64.0 * zoom;
    let header_cy = oy + 8.0 * zoom + header_h / 2.0;
    paint_lucide(
        ui,
        Rect::from_center_size(Pos2::new(ox + 16.0 * zoom, header_cy), Vec2::splat(16.0 * zoom)),
        Lucide::Diamond,
        body_text_color,
    );
    let id_text = format!("#{}", object.id);
    painter.text(
        Pos2::new(ox + 496.0 * zoom, header_cy),
        egui::Align2::RIGHT_CENTER,
        &id_text,
        egui::FontId::new(20.0 * zoom, egui::FontFamily::Proportional),
        body_text_color,
    );
    let title = format!("{} '{}'", object.kind.label_ru(), object.name);
    painter.text(
        Pos2::new(ox + 24.0 * zoom, header_cy),
        egui::Align2::LEFT_CENTER,
        &title,
        egui::FontId::new(20.0 * zoom, egui::FontFamily::Proportional),
        body_text_color,
    );

    let req_rect = Rect::from_min_size(
        Pos2::new(container_x, oy + 72.0 * zoom),
        Vec2::new(container_w, container_h),
    );
    let req_hovered = container_hovered_dir == Some(Direction::Request);
    paint_metric_container(
        ui, painter, object, dynamic, theme, zoom,
        req_hovered,
        mouse_pos,
        Direction::Request, &object.requests, req_rect,
        container_header_h, row_w, row_h, container_x, &row_offsets,
        &mut add_hotspots, &mut menu_hotspots, &mut header_hotspots, &mut drag_hotspots,
    );

    let result_rect = Rect::from_min_size(
        Pos2::new(container_x, oy + 176.0 * zoom),
        Vec2::new(container_w, container_h),
    );
    let result_hovered = container_hovered_dir == Some(Direction::Result);
    paint_metric_container(
        ui, painter, object, dynamic, theme, zoom,
        result_hovered,
        mouse_pos,
        Direction::Result, &object.results, result_rect,
        container_header_h, row_w, row_h, container_x, &row_offsets,
        &mut add_hotspots, &mut menu_hotspots, &mut header_hotspots, &mut drag_hotspots,
    );

    (add_hotspots, menu_hotspots, drag_hotspots, header_hotspots)
}

fn paint_metric_container(
    ui: &egui::Ui,
    painter: &Painter,
    object: &ObjectGeom,
    dynamic: &DynamicState,
    theme: &crate::Theme,
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
    add_hotspots: &mut Vec<(String, Direction, Rect)>,
    menu_hotspots: &mut Vec<(String, Direction, Rect)>,
    header_hotspots: &mut Vec<(String, Direction, Rect)>,
    drag_hotspots: &mut Vec<(String, Direction, String, Rect)>,
) {
    let cx = container_rect.min.x;
    let cy = container_rect.min.y;

    let header_rect = Rect::from_min_size(
        Pos2::new(cx + 0.0 * zoom, cy + 0.0 * zoom),
        Vec2::new(container_rect.width(), container_header_h),
    );
    painter.rect_filled(header_rect, 0.0, theme.card_bg);

    let border_color = if container_hovered { theme.text } else { theme.card_border };
    painter.rect_stroke(
        container_rect,
        0.0,
        Stroke::new(theme.object_border_width * zoom, border_color),
        egui::StrokeKind::Middle,
    );

    let status = worst_status(entries, dynamic);
    let dot_color = status_dot_color(status, dir, theme);
    let label = match dir {
        Direction::Request => "ЗАПРОС",
        Direction::Result => "РЕЗУЛЬТАТ",
    };
    let status_text = status_label_ru(status, dir);
    let problem_count = entries.iter().filter(|e| dynamic.status[e.ix as usize] != StatusCode::Ok).count();
    let status_full = if problem_count > 0 {
        format!("{} ({})", status_text, problem_count)
    } else {
        status_text.to_string()
    };
    let header_text_color = if container_hovered { theme.text } else { theme.text_muted };
    let header_cy = cy + container_header_h / 2.0;

    let header_symbol_color = dot_color;
    paint_lucide(
        ui,
        Rect::from_center_size(Pos2::new(cx + 24.0 * zoom, header_cy), Vec2::splat(16.0 * zoom)),
        Lucide::Diamond,
        header_symbol_color,
    );

    let count_text = format!("{} ({}) {}", label, entries.len(), status_full);
    painter.text(
        Pos2::new(cx + 48.0 * zoom, header_cy),
        egui::Align2::LEFT_CENTER,
        &count_text,
        egui::FontId::new(18.0 * zoom, egui::FontFamily::Proportional),
        header_text_color,
    );

    let menu_rect = Rect::from_min_size(
        Pos2::new(cx + 8.0 * zoom, cy + 8.0 * zoom),
        Vec2::new(32.0 * zoom, container_header_h - 16.0 * zoom),
    );
    menu_hotspots.push((object.id.clone(), dir, menu_rect));
    header_hotspots.push((object.id.clone(), dir, header_rect));

    let visible = entries.len().min(MAX_VISIBLE_METRICS);
    let show_add = visible < MAX_VISIBLE_METRICS;

    let sym_size = (16.0 * zoom).max(10.0 * zoom);
    let (left_icon, right_icon) = match dir {
        Direction::Request => (Lucide::ChevronRight, Lucide::ChevronLeft),
        Direction::Result => (Lucide::ChevronLeft, Lucide::ChevronRight),
    };

    for (i, entry) in entries.iter().enumerate().take(MAX_VISIBLE_METRICS) {
        let row_y = cy + row_offsets[i] * zoom;
        let row_x = cx + 8.0 * zoom;
        let row_rect = Rect::from_min_size(Pos2::new(row_x, row_y), Vec2::new(row_w, row_h));
        let row_hovered = mouse_pos.map_or(false, |pos| row_rect.contains(pos));
        if row_hovered {
            painter.rect_filled(row_rect, 0.0, theme.card_bg);
        }
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

        paint_lucide(
            ui,
            Rect::from_center_size(Pos2::new(cx + 24.0 * zoom, cy_row), Vec2::splat(sym_size)),
            left_icon,
            row_dot_color,
        );
        paint_lucide(
            ui,
            Rect::from_center_size(
                Pos2::new(row_x + row_w - 24.0 * zoom, cy_row),
                Vec2::splat(sym_size),
            ),
            right_icon,
            row_dot_color,
        );
        let row_text_color = if row_hovered { theme.text } else { theme.text_muted };
        painter.text(
            Pos2::new(row_x + TEXT_X_PAD * zoom, cy_row),
            egui::Align2::LEFT_CENTER,
            &entry.name,
            egui::FontId::new(16.0 * zoom, egui::FontFamily::Proportional),
            row_text_color,
        );
        let unit = if entry.unit.is_empty() {
            String::new()
        } else {
            format!(" {}", entry.unit)
        };
        let displayed_value = match dir {
            Direction::Request => dynamic.required[entry.ix as usize].clone(),
            Direction::Result => entry.value.clone(),
        };
        let value_text = format!("{:.0}{}", displayed_value, unit);
        painter.text(
            Pos2::new(row_x + row_w - TEXT_X_PAD * zoom, cy_row),
            egui::Align2::RIGHT_CENTER,
            &value_text,
            egui::FontId::new(16.0 * zoom, egui::FontFamily::Proportional),
            row_text_color,
        );

        drag_hotspots.push((object.id.clone(), dir, entry.name.clone(), row_rect));
    }

    if show_add {
        let add_y = cy + row_offsets[visible] * zoom;
        let add_rect = Rect::from_min_size(Pos2::new(cx + 8.0 * zoom, add_y), Vec2::new(row_w, row_h));
        let add_text = "+ Добавить показатель";
        let add_color = if container_hovered { theme.text } else { theme.text_muted };
        painter.text(
            add_rect.center(),
            egui::Align2::CENTER_CENTER,
            add_text,
            egui::FontId::new(16.0 * zoom, egui::FontFamily::Proportional),
            add_color,
        );
        add_hotspots.push((object.id.clone(), dir, add_rect));
    }
}

/// Пустая пунктирная рамка ("призрачная копия") размером с объект.
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

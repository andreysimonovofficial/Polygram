//! Рисование заголовка контейнера показателей: «ЗАПРОС (N) / ВЫХОД (N) СТАТУС (N)»
//! с точкой статуса и кнопкой ≡.

use egui::{Pos2, Rect, Stroke, Vec2};
use polygram_core::geometry::EntryGeom;
use polygram_core::types::Direction;
use polygram_core::{DynamicState, StatusCode};

use crate::paint::constants::MIN_ROW_H_PX;
use crate::paint::status::worst_status;
use crate::status_dot_color;
use crate::status_label_ru;

/// Рисовать заголовок контейнера: «ЗАПРОС (N) СТАТУС (N)» + точка статуса + кнопка ≡.
/// Возвращает rect кнопки ≡ (если нужно обрабатывать клик).
pub fn paint_container_header(
    painter: &egui::Painter,
    rect: Rect,
    dir: Direction,
    entries: &[EntryGeom],
    dynamic: &DynamicState,
    zoom: f32,
    theme: &crate::theme::Theme,
    is_hovered: bool,
) -> Option<Rect> {
    let label = match dir {
        Direction::Request => "ЗАПРОС",
        Direction::Output => "ВЫХОД",
    };
    let status = worst_status(entries, dynamic);
    let status_text = status_label_ru(status, dir);
    let problem_count = entries.iter().filter(|e| dynamic.status[e.ix as usize] != StatusCode::Ok).count();
    let status_full = if problem_count > 0 {
        format!("{} ({})", status_text, problem_count)
    } else {
        status_text.to_string()
    };
    let header_color = if status != StatusCode::Ok {
        status_dot_color(status, dir, theme)
    } else {
        theme.text_muted
    };
    let text_color = if is_hovered { theme.text } else { header_color };

    // Точка статуса контейнера (слева)
    let dot_color = status_dot_color(status, dir, theme);
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
    let text_w_est = status_full.len() as f32 * 7.0 * zoom;
    let count_text = format!("{} ({}) {}", label, entries.len(), status_full);
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

    let _ = MIN_ROW_H_PX;
    Some(btn_rect)
}

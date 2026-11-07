use crate::theme::Theme;
use egui::{Painter, Rect, Stroke, Vec2};
use polygram_core::types::{Direction, MAX_UI_METRICS_PER_SIDE};
use polygram_core::{DynamicState, ObjectGeom, StatusCode};

pub fn paint_object(
    painter: &Painter,
    object: &ObjectGeom,
    dynamic: &DynamicState,
    theme: &Theme,
    rect_px: Rect,
    cell_px: f32,
    zoom: f32,
    is_hovered: bool,
) {
    // Свечение за объектом (если включено в стиле)
    if theme.glow_enabled {
        let glow_rect = rect_px.expand(theme.glow_size);
        painter.rect_filled(glow_rect, theme.object_corner_radius, theme.glow_color);
    }

    // Тень под объектом (если включена в стиле)
    if theme.shadow_enabled {
        let shadow_rect = rect_px.translate(Vec2::new(theme.shadow_offset[0], theme.shadow_offset[1]));
        painter.rect_filled(shadow_rect, theme.object_corner_radius, theme.shadow_color);
    }

    // Внешняя рамка
    let border_color = if is_hovered {
        theme.text
    } else {
        theme.card_border
    };
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

    // Заголовок
    let header_h = cell_px * theme.object_header_height_cells;
    let header_rect = Rect::from_min_size(rect_px.min, Vec2::new(rect_px.width(), header_h));
    painter.rect_filled(header_rect, 0.0, theme.card_header);

    let title = format!("{} '{}'", object.kind.label_ru(), object.name);
    let name_color = if is_hovered {
        theme.text
    } else {
        theme.text_muted
    };
    painter.text(
        rect_px.min + Vec2::new(8.0 * zoom, header_h / 2.0),
        egui::Align2::LEFT_CENTER,
        &title,
        egui::FontId::new(theme.text_size * zoom, egui::FontFamily::Proportional),
        name_color,
    );

    let id_text = format!("#{}", object.id);
    painter.text(
        rect_px.max - Vec2::new(8.0 * zoom, header_h / 2.0),
        egui::Align2::RIGHT_CENTER,
        &id_text,
        egui::FontId::new(theme.text_size * zoom, egui::FontFamily::Proportional),
        name_color,
    );

    // Два контейнера: ЗАПРОС (верх) и ВЫХОД (низ)
    let container_h = (rect_px.height() - header_h) / 2.0;

    paint_container(
        painter,
        object,
        dynamic,
        theme,
        Rect::from_min_size(
            rect_px.min + Vec2::new(0.0, header_h),
            Vec2::new(rect_px.width(), container_h),
        ),
        cell_px,
        zoom,
        Direction::Request,
        is_hovered,
    );

    paint_container(
        painter,
        object,
        dynamic,
        theme,
        Rect::from_min_size(
            rect_px.min + Vec2::new(0.0, header_h + container_h),
            Vec2::new(rect_px.width(), container_h),
        ),
        cell_px,
        zoom,
        Direction::Output,
        is_hovered,
    );
}

/// Нарисовать контейнер показателей (Запрос или Выход).
pub fn paint_container(
    painter: &Painter,
    object: &ObjectGeom,
    dynamic: &DynamicState,
    theme: &Theme,
    rect: Rect,
    cell_px: f32,
    zoom: f32,
    dir: Direction,
    is_obj_hovered: bool,
) -> Option<Rect> {
    let entries = match dir {
        Direction::Request => &object.requests,
        Direction::Output => &object.outputs,
    };

    let header_h = cell_px * theme.container_header_height_cells;

    // Статус контейнера (худший среди записей)
    let mut worst_rank: u8 = 0;
    for e in entries {
        let s = dynamic.status[e.ix as usize];
        let r = match s {
            StatusCode::Ok => 0,
            StatusCode::Unknown | StatusCode::Disabled => 1,
            StatusCode::Unconnected => 2,
            StatusCode::Warning => 3,
            StatusCode::Deficit => 4,
            StatusCode::Surplus => 5,
            StatusCode::Invalid => 6,
        };
        if r > worst_rank {
            worst_rank = r;
        }
    }
    let container_status = match worst_rank {
        4 => StatusCode::Deficit,
        5 => StatusCode::Surplus,
        3 => StatusCode::Warning,
        2 => StatusCode::Unconnected,
        6 => StatusCode::Invalid,
        _ => StatusCode::Ok,
    };

    let dot_color = super::status_dot_color(container_status, theme);
    let label = match dir {
        Direction::Request => "ЗАПРОС",
        Direction::Output => "ВЫХОД",
    };
    let status_text = super::status_label_ru(container_status, dir);

    // Количество записей с проблемным статусом (не Ok).
    let problem_count = entries
        .iter()
        .filter(|e| dynamic.status[e.ix as usize] != StatusCode::Ok)
        .count();
    let status_full = if problem_count > 0 {
        format!("{} ({})", status_text, problem_count)
    } else {
        status_text.to_string()
    };
    let count_text = format!("{} ({}) {}", label, entries.len(), status_full);

    // Цвет текста заголовка контейнера: красный при проблеме, иначе серый.
    let header_text_color = if container_status != StatusCode::Ok {
        super::status_dot_color(container_status, theme)
    } else {
        theme.text_muted
    };

    // Заголовок контейнера
    let header_rect = Rect::from_min_size(rect.min, Vec2::new(rect.width(), header_h));
    painter.rect_filled(header_rect, 0.0, theme.card_bg);

    // Точка статуса (радиус порта из стиля)
    let dot_radius = theme.port_radius.max(2.0) * zoom;
    painter.circle_filled(rect.min + Vec2::new(8.0 * zoom, header_h / 2.0), dot_radius, dot_color);

    // Кнопка списка (справа) — масштабируется с зумом.
    let btn_size = 24.0 * zoom;
    let btn_rect = Rect::from_min_size(
        rect.max - Vec2::new(btn_size, header_h),
        Vec2::new(btn_size, header_h),
    );

    // Доступная ширина для текста заголовка (от 16px отступа до кнопки с зазором).
    let header_avail_w = (btn_rect.min.x - rect.min.x - 32.0 * zoom).max(0.0);
    let header_text_width = count_text.len() as f32 * 7.0 * zoom;
    let header_text_display: String = if header_text_width > header_avail_w && header_avail_w > 14.0 * zoom
    {
        let n_chars = (header_avail_w / (7.0 * zoom)) as usize;
        if n_chars > 4 {
            count_text.chars().take(n_chars - 4).collect::<String>() + "..."
        } else {
            "...".to_string()
        }
    } else {
        count_text.clone()
    };

    let header_text_actual = if is_obj_hovered {
        theme.text
    } else {
        header_text_color
    };
    painter.text(
        rect.min + Vec2::new(16.0 * zoom, header_h / 2.0),
        egui::Align2::LEFT_CENTER,
        &header_text_display,
        egui::FontId::new(theme.text_size * zoom, egui::FontFamily::Proportional),
        header_text_actual,
    );

    painter.rect_stroke(btn_rect, 1.0 * zoom, Stroke::new(1.0_f32 * zoom, theme.card_border));
    let btn_text_actual = if is_obj_hovered {
        theme.text
    } else {
        header_text_color
    };
    painter.text(
        btn_rect.center(),
        egui::Align2::CENTER_CENTER,
        "≡",
        egui::FontId::new(theme.text_size * zoom, egui::FontFamily::Proportional),
        btn_text_actual,
    );

    // Записи и строка добавления.
    let show_add = entries.len() < MAX_UI_METRICS_PER_SIDE;
    let n_rows = entries.len() + usize::from(show_add);
    let n_rows = n_rows.max(1);
    let entry_h = (rect.height() - header_h) / n_rows as f32;

    let mut add_rect: Option<Rect> = None;

    if entries.is_empty() {
        if show_add {
            let r = Rect::from_min_size(
                rect.min + Vec2::new(0.0, header_h),
                Vec2::new(rect.width(), entry_h),
            );
            paint_add_row(painter, r, zoom, is_obj_hovered, theme);
            add_rect = Some(r);
        }
    } else {
        for (k, entry) in entries.iter().enumerate() {
            let entry_y = rect.min.y + header_h + (k as f32 + 0.5) * entry_h;
            let entry_rect = Rect::from_min_size(
                rect.min + Vec2::new(0.0, header_h + k as f32 * entry_h),
                Vec2::new(rect.width(), entry_h),
            );

            let status = dynamic.status[entry.ix as usize];
            let entry_dot_color = super::status_dot_color(status, theme);

            let value_text = format!("{:.0}", entry.value);

            // Фон строки при наведении на объект
            if is_obj_hovered {
                painter.rect_filled(entry_rect, 0.0, theme.card_header);
            }

            // Точка статуса записи
            painter.circle_filled(rect.min + Vec2::new(8.0 * zoom, entry_y), dot_radius, entry_dot_color);

            // Имя записи
            let name_color = if is_obj_hovered {
                theme.text
            } else {
                theme.text_muted
            };
            painter.text(
                rect.min + Vec2::new(16.0 * zoom, entry_y),
                egui::Align2::LEFT_CENTER,
                &entry.name,
                egui::FontId::new(theme.text_size * zoom, egui::FontFamily::Proportional),
                name_color,
            );

            // Значение (справа)
            painter.text(
                entry_rect.max - Vec2::new(8.0 * zoom, 0.0),
                egui::Align2::RIGHT_CENTER,
                &value_text,
                egui::FontId::new(theme.text_size * zoom, egui::FontFamily::Proportional),
                name_color,
            );
        }
        if show_add {
            let r = Rect::from_min_size(
                rect.min + Vec2::new(0.0, header_h + entries.len() as f32 * entry_h),
                Vec2::new(rect.width(), entry_h),
            );
            paint_add_row(painter, r, zoom, is_obj_hovered, theme);
            add_rect = Some(r);
        }
    }

    add_rect
}

/// Строка "+ Добавить показатель" внутри контейнера.
fn paint_add_row(
    painter: &Painter,
    rect: Rect,
    zoom: f32,
    is_hovered: bool,
    theme: &Theme,
) {
    let add_text = "+ Добавить показатель";
    let add_color = if is_hovered {
        theme.text
    } else {
        theme.text_muted
    };
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        add_text,
        egui::FontId::new(theme.text_size * zoom, egui::FontFamily::Proportional),
        add_color,
    );
}

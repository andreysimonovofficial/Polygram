use crate::theme::Theme;
use egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Stroke, Vec2};
use polygram_core::{DynamicState, ObjectGeom, StatusCode};
use polygram_core::types::Direction;

/// Высокопроизводительный рендер объекта "Процесс" по макету design.json.
/// Особенности:
/// - Прозрачный фон (нет rect_filled карточки), только обводка.
/// - Минимизировано число format! и аллокаций в горячем пути.
pub fn paint_process(
    painter: &Painter,
    object: &ObjectGeom,
    dynamic: &DynamicState,
    theme: &Theme,
    rect_px: Rect,
    cell_px: f32,
    is_hovered: bool,
) {
    let stroke_outer = Color32::from_rgb(80, 80, 80);
    let stroke_inner = Color32::from_rgb(60, 60, 60);
    let text_main = Color32::from_rgb(230, 230, 235);
    let text_muted = Color32::from_rgb(115, 116, 120);
    let dot_radius = 3.0;

    // Заголовок объекта
    let header_h = cell_px * 1.0;

    // Внешняя рамка (только обводка, без заливки)
    painter.rect_stroke(
        rect_px,
        0.0,
        Stroke::new(1.0_f32, stroke_outer),
    );

    // Разделительная линия под заголовком
    let line_y = rect_px.min.y + header_h;
    painter.line_segment(
        [Pos2::new(rect_px.min.x, line_y), Pos2::new(rect_px.max.x, line_y)],
        Stroke::new(1.0_f32, stroke_outer),
    );

    // Название объекта (влево, белый, полужирный)
    let title = format!("Процесс ‘{}’", object.name);
    painter.text(
        rect_px.min + Vec2::new(11.0, header_h / 2.0),
        Align2::LEFT_CENTER,
        title,
        FontId::new(15.0, egui::FontFamily::Proportional),
        text_main,
    );

    // ID объекта (вправо, серый, полужирный)
    let id_text = format!("#{}", object.ix);
    painter.text(
        rect_px.max - Vec2::new(11.0, header_h / 2.0),
        Align2::RIGHT_CENTER,
        id_text,
        FontId::new(15.0, egui::FontFamily::Proportional),
        text_muted,
    );

    // Два контейнера: ЗАПРОС (верх) и ВЫХОД (низ)
    let container_h = (rect_px.height() - header_h) / 2.0;
    let ch = cell_px * 0.5; // высота заголовка контейнера

    for (dir, container_top) in [
        (Direction::Request, rect_px.min.y + header_h),
        (Direction::Output, rect_px.min.y + header_h + container_h),
    ] {
        let container_rect = Rect::from_min_size(
            Pos2::new(rect_px.min.x, container_top),
            Vec2::new(rect_px.width(), container_h),
        );

        // Обводка контейнера (без заливки)
        painter.rect_stroke(
            container_rect,
            0.0,
            Stroke::new(1.0_f32, stroke_inner),
        );

        let entries = match dir {
            Direction::Request => &object.requests,
            Direction::Output => &object.outputs,
        };

        // Худший статус среди записей контейнера
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

        let dot_color = match worst_rank {
            4 => theme.status_deficit,
            5 => theme.status_surplus,
            3 => theme.status_warning,
            2 => theme.status_unconnected,
            6 => theme.status_invalid,
            _ => text_muted,
        };

        // Точка статуса (слева)
        painter.circle_filled(
            container_rect.min + Vec2::new(8.0, ch / 2.0),
            dot_radius,
            dot_color,
        );

        // Текст статуса контейнера
        let status_label = match dir {
            Direction::Request => "ЗАПРОС",
            Direction::Output => "ВЫХОД",
        };
        let status_text = match worst_rank {
            4 => "ОЖИДАНИЕ",
            5 => "ИЗЫТОК",
            3 => "ПРЕДУПРЕЖДЕНИЕ",
            2 => "НЕ ПОДКЛЮЧЕНО",
            6 => "ОШИБКА",
            _ => match dir {
                Direction::Request => "УДОВЛЕТВОРЁН",
                Direction::Output => "ОБЕСПЕЧЕН",
            },
        };

        let problem_count = entries
            .iter()
            .filter(|e| dynamic.status[e.ix as usize] != StatusCode::Ok)
            .count();
        let status_full = if problem_count > 0 {
            format!("{} ({})", status_text, problem_count)
        } else {
            status_text.to_string()
        };
        let count_text = format!("{} ({}) {}", status_label, entries.len(), status_full);

        let header_text_color = if worst_rank != 0 { dot_color } else { text_muted };

        painter.text(
            container_rect.min + Vec2::new(16.0, ch / 2.0),
            Align2::LEFT_CENTER,
            count_text,
            FontId::default(),
            header_text_color,
        );

        // Кнопка меню "≡" (справа)
        let btn_w = 24.0;
        let btn_rect = Rect::from_min_size(
            container_rect.max - Vec2::new(btn_w, ch),
            Vec2::new(btn_w, ch),
        );
        painter.rect_stroke(
            btn_rect,
            0.0,
            Stroke::new(1.0_f32, stroke_inner),
        );
        painter.text(
            btn_rect.center(),
            Align2::CENTER_CENTER,
            "≡",
            FontId::default(),
            header_text_color,
        );

        // Содержимое контейнера (пустое состояние или записи)
        if entries.is_empty() {
            let add_text = "+ Добавить показатель";
            let add_color = if is_hovered { text_main } else { text_muted };
            let add_rect = Rect::from_min_size(
                container_rect.min + Vec2::new(16.0, ch),
                Vec2::new(container_rect.width() - 32.0, container_h - ch),
            );
            painter.text(
                add_rect.center(),
                Align2::CENTER_CENTER,
                add_text,
                FontId::default(),
                add_color,
            );
        } else {
            let entry_h = (container_h - ch) / entries.len() as f32;
            for (k, entry) in entries.iter().enumerate() {
                let entry_y = container_rect.min.y + ch + (k as f32 + 0.5) * entry_h;
                let entry_rect = Rect::from_min_size(
                    container_rect.min + Vec2::new(0.0, ch + k as f32 * entry_h),
                    Vec2::new(container_rect.width(), entry_h),
                );

                let status = dynamic.status[entry.ix as usize];
                let entry_dot_color = match status {
                    StatusCode::Ok => text_muted,
                    StatusCode::Warning => theme.status_warning,
                    StatusCode::Deficit => theme.status_deficit,
                    StatusCode::Surplus => theme.status_surplus,
                    StatusCode::Unconnected => theme.status_unconnected,
                    StatusCode::Invalid => theme.status_invalid,
                    StatusCode::Disabled => text_muted,
                    StatusCode::Unknown => text_muted,
                };

                if is_hovered {
                    painter.rect_filled(
                        entry_rect,
                        0.0,
                        Color32::from_rgb(45, 45, 54),
                    );
                }

                painter.circle_filled(
                    container_rect.min + Vec2::new(8.0, entry_y),
                    dot_radius,
                    entry_dot_color,
                );

                let name_color = if is_hovered { text_main } else { text_muted };
                painter.text(
                    container_rect.min + Vec2::new(16.0, entry_y),
                    Align2::LEFT_CENTER,
                    entry.name.clone(),
                    FontId::default(),
                    name_color,
                );

                let value_text = format!("{:.0}", entry.value);
                painter.text(
                    entry_rect.max - Vec2::new(8.0, 0.0),
                    Align2::RIGHT_CENTER,
                    value_text,
                    FontId::default(),
                    name_color,
                );
            }
        }
    }
}

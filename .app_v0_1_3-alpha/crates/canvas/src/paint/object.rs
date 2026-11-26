//! Рисование объекта "Процесс" на холсте.
//! Реализует динамическую геометрию по CSS спецификации:
//! Процесс растягивается под контейнеры, Контейнер растягивается под показатели.

use egui::{Painter, Pos2, Rect, Stroke, Vec2, Align2, FontId, FontFamily, StrokeKind, Color32};
use polygram_core::types::Direction;
use polygram_core::{DynamicState, EntryGeom, ObjectGeom, StatusCode};

use crate::icons::{default_icon_for, paint_lucide};
use crate::paint::constants::{
    CONTAINER_BORDER_RADIUS_PX, CONTAINER_HEADER_H_PX, CONTAINER_PADDING_PX, GAP_PX, ICON_SIZE_PX,
    PROCESS_BORDER_RADIUS_PX, PROCESS_HEADER_H_PX, PROCESS_PADDING_PX, ROW_BORDER_RADIUS_PX,
    ROW_H_PX, ROW_PADDING_X_PX,
};
use crate::paint::status::worst_status;
use crate::status_label_ru;
use egui_lucide::Lucide;

/// Результат отрисовки процесса: общая высота карточки и все горячие зоны.
pub struct ProcessPaintResult {
    /// Общая высота карточки процесса в пикселях (при текущем zoom).
    pub total_height: f32,
    /// Горячие зоны добавления показателя (id объекта, направление, экранный rect).
    pub add_hotspots: Vec<(String, Direction, Rect)>,
    /// Горячие зоны кнопки меню контейнера (id объекта, направление, экранный rect кнопки ≡).
    pub menu_hotspots: Vec<(String, Direction, Rect)>,
    /// Горячие зоны строк показателей для перетаскивания (id объекта, направление, имя показателя, экранный rect строки).
    pub drag_hotspots: Vec<(String, Direction, String, Rect)>,
    /// Горячие зоны заголовка контейнера (id объекта, направление, экранный rect шапки).
    pub header_hotspots: Vec<(String, Direction, Rect)>,
}

/// Вычислить общую высоту процесса в пикселях (при заданном zoom и ширине карточки).
pub fn process_total_height(
    object: &ObjectGeom,
    _dynamic: &DynamicState,
    zoom: f32,
    _card_w: f32,
) -> f32 {
    let padding = PROCESS_PADDING_PX * zoom;
    let gap = GAP_PX * zoom;
    let header_h = PROCESS_HEADER_H_PX * zoom;
    let container_header_h = CONTAINER_HEADER_H_PX * zoom;
    let row_h = ROW_H_PX * zoom;
    let container_padding = CONTAINER_PADDING_PX * zoom;

    let req_container_h =
        container_total_height(object.requests.len(), container_header_h, row_h, gap, container_padding);
    let result_container_h =
        container_total_height(object.results.len(), container_header_h, row_h, gap, container_padding);

    padding + header_h + gap + req_container_h + gap + result_container_h + padding
}

/// Вычислить общую высоту контейнера: шапка + строки показателя + gap между строками + паддинги.
/// Для пустого контейнера добавляется строка "+ Добавить показатель" (row_h).
fn container_total_height(n_entries: usize, header_h: f32, row_h: f32, gap: f32, padding: f32) -> f32 {
    if n_entries == 0 {
        padding + header_h + row_h + padding
    } else {
        padding + header_h + n_entries as f32 * row_h + (n_entries - 1) as f32 * gap + padding
    }
}

/// Высокопроизводительный процедурный рендер объекта "Процесс" по динамической CSS-геометрии.
/// Карточка: заголовок (Имя + #ID + ◆) → контейнер Запрос → контейнер Результат.
/// Каждый контейнер: шапка (LABEL (N) СТАТУС + ◆) + строки показателя.
/// Процесс растягивается под высоту контейнеров, Контейнер растягивается под количество показателей.
/// Возвращает общую высоту карточки и все горячие зоны.
pub fn paint_process(
    ui: &egui::Ui,
    painter: &Painter,
    object: &ObjectGeom,
    dynamic: &DynamicState,
    theme: &crate::Theme,
    ox: f32,
    oy: f32,
    card_w: f32,
    zoom: f32,
    mouse_pos: Option<Pos2>,
    is_selected: bool,
) -> ProcessPaintResult {
    let mut add_hotspots: Vec<(String, Direction, Rect)> = Vec::new();
    let mut menu_hotspots: Vec<(String, Direction, Rect)> = Vec::new();
    let mut header_hotspots: Vec<(String, Direction, Rect)> = Vec::new();
    let mut drag_hotspots: Vec<(String, Direction, String, Rect)> = Vec::new();

    // CSS константы (масштабируются под zoom).
    let padding = PROCESS_PADDING_PX * zoom;
    let gap = GAP_PX * zoom;
    let header_h = PROCESS_HEADER_H_PX * zoom;
    let container_header_h = CONTAINER_HEADER_H_PX * zoom;
    let row_h = ROW_H_PX * zoom;
    let icon_size = ICON_SIZE_PX * zoom;
    let container_padding = CONTAINER_PADDING_PX * zoom;
    let process_radius = PROCESS_BORDER_RADIUS_PX * zoom;

    let total_height = process_total_height(object, dynamic, zoom, card_w);

    let card_rect = Rect::from_min_size(Pos2::new(ox, oy), Vec2::new(card_w, total_height));

        // === Обложка процесса (когда не наведено) ===
        let is_hovered = mouse_pos.map_or(false, |p| card_rect.contains(p));
        if !is_hovered {
            // Фон полностью перекрывает площадь процесса.
            painter.rect_filled(card_rect, process_radius, Color32::from_rgb(32, 32, 32));
            // Контур обложки.
            let border_width = 1.0 * zoom;
            painter.rect_stroke(
                card_rect,
                process_radius,
                Stroke::new(border_width, Color32::from_rgb(55, 56, 57)),
                StrokeKind::Middle,
            );
            // Масштабирование содержимого обложки при отдалении камеры (RTS-принцип):
            // при zoom < 1.0 содержимое увеличивается, чтобы остаться читабельным издалека.
            // Содержимое не ограничено рамкой карточки объекта и может выходить за её пределы.
            let cover_scale = if zoom < 1.0 { 1.0 / zoom } else { 1.0 };
            // Иконка обложки: 10 клеток / 3, строго по центру, белый цвет.
            let cover_icon_side = (10.0 / 3.0) * (theme.cell_size * zoom) * cover_scale;
            let cover_icon = default_icon_for(object.kind, &object.icon);
            let center = card_rect.center();

            // Имя объекта и #ID — в одну горизонтальную строку с иконкой,
            // весь ряд центрируется относительно центра обложки.
            let name_sig = format!("{}", object.name);
            let id_sig = format!("#{}", object.id);
            let name_size = 13.0 * zoom * cover_scale;
            let id_size = 11.0 * zoom * cover_scale;
            let name_color = Color32::WHITE;
            let id_color = Color32::from_rgb(0xA1, 0xA1, 0xAA);
            let gap = 8.0 * zoom * cover_scale;

            let name_font = FontId::new(name_size, FontFamily::Name("inter_semibold".into()));
            let id_font = FontId::new(id_size, FontFamily::Name("inter_medium".into()));

            // Приблизительная ширина текста (Inter, оценка по размеру шрифта и длине строки).
            let name_w = name_sig.len() as f32 * name_size * 0.58;
            let id_text_w = id_sig.len() as f32 * id_size * 0.52;
            let pill_h = id_size * 1.4;
            let pill_w = (id_text_w + 8.0 * zoom * cover_scale).max(id_size * 2.0);

            let row_w = cover_icon_side + gap + name_w + gap + pill_w;
            let start_x = center.x - row_w / 2.0;

            // Иконка — слева, по вертикальному центру обложки.
            let icon_rect = Rect::from_center_size(
                Pos2::new(start_x + cover_icon_side / 2.0, center.y),
                Vec2::splat(cover_icon_side),
            );
            paint_lucide(ui, icon_rect, cover_icon, Color32::WHITE);

            // Название объекта — по центру строки, по вертикальному центру обложки.
            painter.text(
                Pos2::new(start_x + cover_icon_side + gap + name_w / 2.0, center.y),
                Align2::CENTER_CENTER,
                &name_sig,
                name_font,
                name_color,
            );

            // Плашка #ID — справа, по вертикальному центру обложки.
            let pill_rect = Rect::from_center_size(
                Pos2::new(
                    start_x + cover_icon_side + gap + name_w + gap + pill_w / 2.0,
                    center.y,
                ),
                Vec2::new(pill_w, pill_h),
            );
            painter.rect_filled(
                pill_rect,
                2.0 * zoom * cover_scale,
                Color32::from_rgb(0x25, 0x25, 0x25),
            );
            painter.rect_stroke(
                pill_rect,
                2.0 * zoom * cover_scale,
                Stroke::new(1.0 * zoom * cover_scale, Color32::from_rgb(0x37, 0x38, 0x39)),
                StrokeKind::Middle,
            );
            painter.text(
                pill_rect.center(),
                Align2::CENTER_CENTER,
                &id_sig,
                id_font,
                id_color,
            );
            return ProcessPaintResult {
            total_height,
            add_hotspots: Vec::new(),
            menu_hotspots: Vec::new(),
            drag_hotspots: Vec::new(),
            header_hotspots: Vec::new(),
        };
    }

    // Фон карточки процесса #202020.
    painter.rect_filled(card_rect, process_radius, Color32::from_rgb(32, 32, 32));

    // Граница карточки процесса #373839.
    let border_width = if is_selected { 2.0 * zoom } else { 1.0 * zoom };
    painter.rect_stroke(
        card_rect,
        process_radius,
        Stroke::new(border_width, Color32::from_rgb(55, 56, 57)),
        StrokeKind::Middle,
    );

    // === Шапка процесса ===
    let header_cy = oy + header_h / 2.0;

    // Иконка типа процесса (book-up-2).
    let process_status = match (
        worst_status(&object.requests, dynamic),
        worst_status(&object.results, dynamic),
    ) {
        (StatusCode::Expected, _) | (_, StatusCode::Expected) => StatusCode::Expected,
        (StatusCode::Surplus, _) | (_, StatusCode::Surplus) => StatusCode::Surplus,
        _ => StatusCode::Ok,
    };
    let (process_type_color, process_waiting_color, process_surplus_color) = match process_status {
        StatusCode::Expected => (theme.status_deficit, theme.status_deficit, theme.text_muted),
        StatusCode::Surplus => (theme.status_warning, theme.text_muted, theme.status_warning),
        _ => (theme.text_muted, theme.text_muted, theme.text_muted),
    };
    paint_lucide(
        ui,
        Rect::from_center_size(Pos2::new(ox + padding + icon_size / 2.0, header_cy), Vec2::splat(icon_size)),
        Lucide::BookUp2,
        process_type_color,
    );

    // Название процесса.
    let title = object.name.clone();
    let title_x = ox + padding + icon_size + 8.0 * zoom;
    painter.text(
        Pos2::new(title_x, header_cy),
        Align2::LEFT_CENTER,
        &title,
        FontId::new(20.0 * zoom, FontFamily::Proportional),
        theme.text,
    );

    // #ID процесса (справа, до иконок статуса).
    // Регуем место справа: padding + surplus_icon + gap + waiting_icon + gap
    let id_right = ox + card_w - padding - 2.0 * icon_size - 2.0 * gap;
    let id_text = format!("#{}", object.id);
    painter.text(
        Pos2::new(id_right, header_cy),
        Align2::RIGHT_CENTER,
        &id_text,
        FontId::new(20.0 * zoom, FontFamily::Proportional),
        theme.text_muted,
    );

    // Иконка Ожидание (clock-exclamation-point) — перед иконкой избытка.
    let surplus_cx = ox + card_w - padding - icon_size / 2.0;
    let waiting_cx = surplus_cx - icon_size - gap;
    paint_lucide(
        ui,
        Rect::from_center_size(Pos2::new(waiting_cx, header_cy), Vec2::splat(icon_size)),
        Lucide::Clock,
        process_waiting_color,
    );

    // Иконка Избыток (square-exclamation-point) — крайнюю справа.
    paint_lucide(
        ui,
        Rect::from_center_size(Pos2::new(surplus_cx, header_cy), Vec2::splat(icon_size)),
        Lucide::SquareExclamationPoint,
        process_surplus_color,
    );

    // === Контейнеры ===
    let container_x = ox + padding;
    let container_w = card_w - 2.0 * padding;

    let mut y = oy + padding + header_h + gap;

    // Контейнер ЗАПРОС.
    let req_container_h =
        container_total_height(object.requests.len(), container_header_h, row_h, gap, container_padding);
    let req_rect = Rect::from_min_size(
        Pos2::new(container_x, y),
        Vec2::new(container_w, req_container_h),
    );
    paint_metric_container(
        ui, painter, object, dynamic, theme, zoom,
        mouse_pos, Direction::Request, &object.requests,
        req_rect, container_header_h, row_h, gap, container_padding,
        &mut add_hotspots, &mut menu_hotspots, &mut header_hotspots, &mut drag_hotspots,
    );
    y += req_container_h + gap;

    // Контейнер РЕЗУЛЬТАТ.
    let result_container_h =
        container_total_height(object.results.len(), container_header_h, row_h, gap, container_padding);
    let result_rect = Rect::from_min_size(
        Pos2::new(container_x, y),
        Vec2::new(container_w, result_container_h),
    );
    paint_metric_container(
        ui, painter, object, dynamic, theme, zoom,
        mouse_pos, Direction::Result, &object.results,
        result_rect, container_header_h, row_h, gap, container_padding,
        &mut add_hotspots, &mut menu_hotspots, &mut header_hotspots, &mut drag_hotspots,
    );

    ProcessPaintResult {
        total_height,
        add_hotspots,
        menu_hotspots,
        drag_hotspots,
        header_hotspots,
    }
}

/// Отрисовать контейнер показателей: шапка + строки.
fn paint_metric_container(
    ui: &egui::Ui,
    painter: &Painter,
    object: &ObjectGeom,
    dynamic: &DynamicState,
    theme: &crate::Theme,
    zoom: f32,
    mouse_pos: Option<Pos2>,
    dir: Direction,
    entries: &[EntryGeom],
    container_rect: Rect,
    container_header_h: f32,
    row_h: f32,
    gap: f32,
    padding: f32,
    add_hotspots: &mut Vec<(String, Direction, Rect)>,
    menu_hotspots: &mut Vec<(String, Direction, Rect)>,
    header_hotspots: &mut Vec<(String, Direction, Rect)>,
    drag_hotspots: &mut Vec<(String, Direction, String, Rect)>,
) {
    let cx = container_rect.min.x;
    let cy = container_rect.min.y;
    let cw = container_rect.width();
    let icon_size = ICON_SIZE_PX * zoom;
    let row_pad = ROW_PADDING_X_PX * zoom;

    // Фон контейнера #222222.
    let container_radius = CONTAINER_BORDER_RADIUS_PX * zoom;
    painter.rect_filled(container_rect, container_radius, Color32::from_rgb(34, 34, 34));

    // Граница контейнера #373839.
    painter.rect_stroke(
        container_rect,
        container_radius,
        Stroke::new(1.0 * zoom, Color32::from_rgb(55, 56, 57)),
        StrokeKind::Middle,
    );

    // === Шапка контейнера ===
    let header_rect = Rect::from_min_size(
        Pos2::new(cx, cy),
        Vec2::new(cw, container_header_h),
    );
    let header_cy = cy + container_header_h / 2.0;

    // Текст шапки: "ЗАПРОС (N) СТАТУС" / "РЕЗУЛЬТАТ (N) СТАТУС".
    let label = match dir {
        Direction::Request => "ЗАПРОС",
        Direction::Result => "РЕЗУЛЬТАТ",
    };
    let status = worst_status(entries, dynamic);
    let status_text = status_label_ru(status, dir);
    let (container_type_color, container_waiting_color, container_surplus_color) = match status {
        StatusCode::Expected => (theme.status_deficit, theme.status_deficit, theme.text_muted),
        StatusCode::Surplus => (theme.status_warning, theme.text_muted, theme.status_warning),
        _ => (theme.text_muted, theme.text_muted, theme.text_muted),
    };
    let header_text = format!("{} ({}) {}", label, entries.len(), status_text);

    // Иконка типа контейнера (package-check).
    paint_lucide(
        ui,
        Rect::from_center_size(Pos2::new(cx + padding + icon_size / 2.0, header_cy), Vec2::splat(icon_size)),
        Lucide::PackageCheck,
        container_type_color,
    );

    // Текст шапки: после иконки типа + gap.
    let header_text_x = cx + padding + icon_size + 8.0 * zoom;
    painter.text(
        Pos2::new(header_text_x, header_cy),
        Align2::LEFT_CENTER,
        &header_text,
        FontId::new(18.0 * zoom, FontFamily::Proportional),
        theme.text,
    );

    // Иконка Ожидание (clock-exclamation-point) — перед иконкой избытка.
    // Справа: padding + surplus_icon + gap + waiting_icon + gap
    let surplus_cx = cx + cw - padding - icon_size / 2.0;
    let waiting_cx = surplus_cx - icon_size - gap;
    paint_lucide(
        ui,
        Rect::from_center_size(Pos2::new(waiting_cx, header_cy), Vec2::splat(icon_size)),
        Lucide::Clock,
        container_waiting_color,
    );

    // Иконка Избыток (square-exclamation-point) — крайнюю справа.
    paint_lucide(
        ui,
        Rect::from_center_size(Pos2::new(surplus_cx, header_cy), Vec2::splat(icon_size)),
        Lucide::SquareExclamationPoint,
        container_surplus_color,
    );

    // Горячая зона кнопки меню ≡ (слева, поверх иконки типа).
    let menu_rect = Rect::from_min_size(
        Pos2::new(cx + padding, cy + padding),
        Vec2::new(32.0 * zoom, container_header_h - 2.0 * padding),
    );
    menu_hotspots.push((object.id.clone(), dir, menu_rect));
    header_hotspots.push((object.id.clone(), dir, header_rect));

    // === Показатели (строки) ===
    let mut row_y = cy + padding + container_header_h;

    for entry in entries.iter() {
        // Строка показателя: от container_x + padding, ширина container_w - 2*padding.
        let row_left = cx + padding;
        let row_right = cx + cw - padding;
        let row_rect = Rect::from_min_size(
            Pos2::new(row_left, row_y),
            Vec2::new(cw - 2.0 * padding, row_h),
        );
        let row_hovered = mouse_pos.map_or(false, |pos| row_rect.contains(pos));

        // Фон строки показателя #252525 (всегда).
        painter.rect_filled(row_rect, row_border_rad(zoom), Color32::from_rgb(37, 37, 37));

        // Граница строки #373839 (всегда).
        painter.rect_stroke(
            row_rect,
            row_border_rad(zoom),
            Stroke::new(1.0 * zoom, Color32::from_rgb(55, 56, 57)),
            StrokeKind::Middle,
        );

        let cy_row = row_rect.center().y;

        // Цвет иконок строки по статусу показателя.
        let status = dynamic.status[entry.ix as usize];
        let (type_color, waiting_color, surplus_color) = match status {
            StatusCode::Expected => (theme.status_deficit, theme.status_deficit, theme.text_muted),
            StatusCode::Surplus => (theme.status_warning, theme.text_muted, theme.status_warning),
            _ => (theme.text_muted, theme.text_muted, theme.text_muted),
        };

        // Content area: после left padding (16px) и перед right padding (16px).
        let content_left = row_left + row_pad;   // +16px от левого края строки
        let content_right = row_right - row_pad; // -16px от правого края строки

        // Иконка типа показателя — слева, по левому краю контент-зоны.
        let icon_cx = content_left + icon_size / 2.0;
        paint_lucide(
            ui,
            Rect::from_center_size(Pos2::new(icon_cx, cy_row), Vec2::splat(icon_size)),
            Lucide::List,
            type_color,
        );

        // Имя показателя — начинается после иконки + gap.
        let name_x = content_left + icon_size + gap;
        painter.text(
            Pos2::new(name_x, cy_row),
            Align2::LEFT_CENTER,
            &entry.name,
            FontId::new(16.0 * zoom, FontFamily::Proportional),
            if row_hovered { theme.text } else { theme.text_muted },
        );

        // Значение + единица — справа, перед иконками статуса.
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

        // Правый край значения: от content_right отсчитываем surplus+gap+waiting+gap.
        let value_right = content_right - icon_size - gap - icon_size - gap;
        painter.text(
            Pos2::new(value_right, cy_row),
            Align2::RIGHT_CENTER,
            &value_text,
            FontId::new(16.0 * zoom, FontFamily::Proportional),
            if row_hovered { theme.text } else { theme.text_muted },
        );

        // Иконка Ожидание (clock-exclamation-point).
        let waiting_cx = content_right - icon_size - gap - icon_size / 2.0;
        paint_lucide(
            ui,
            Rect::from_center_size(Pos2::new(waiting_cx, cy_row), Vec2::splat(icon_size)),
            Lucide::Clock,
            waiting_color,
        );

        // Иконка Избыток (square-exclamation-point) — крайнюю справа.
        let surplus_cx = content_right - icon_size / 2.0;
        paint_lucide(
            ui,
            Rect::from_center_size(Pos2::new(surplus_cx, cy_row), Vec2::splat(icon_size)),
            Lucide::SquareExclamationPoint,
            surplus_color,
        );

        // Горячая зона перетаскивания строки.
        drag_hotspots.push((object.id.clone(), dir, entry.name.clone(), row_rect));

        row_y += row_h + gap;
    }

    // Строка "+ Добавить показатель" только если контейнер пуст.
    // Стилизуется как строка показателя: рамка, паддинг, текст по центру.
    let show_add = entries.is_empty();
    if show_add {
        let add_rect = Rect::from_min_size(
            Pos2::new(cx + padding, row_y),
            Vec2::new(cw - 2.0 * padding, row_h),
        );
        let add_hovered = mouse_pos.map_or(false, |pos| add_rect.contains(pos));

        // Фон строки "+ Добавить показатель" #252525 (всегда).
        painter.rect_filled(add_rect, row_border_rad(zoom), Color32::from_rgb(37, 37, 37));

        // Граница строки #373839 (всегда).
        painter.rect_stroke(
            add_rect,
            row_border_rad(zoom),
            Stroke::new(1.0 * zoom, Color32::from_rgb(55, 56, 57)),
            StrokeKind::Middle,
        );

        let cy_row = add_rect.center().y;

        // Текст "+ Добавить показатель" — по центру строки.
        painter.text(
            Pos2::new(add_rect.center().x, cy_row),
            Align2::CENTER_CENTER,
            "+ Добавить показатель",
            FontId::new(16.0 * zoom, FontFamily::Proportional),
            if add_hovered { theme.text } else { theme.text_muted },
        );

        add_hotspots.push((object.id.clone(), dir, add_rect));
    }
}

fn row_border_rad(zoom: f32) -> f32 {
    ROW_BORDER_RADIUS_PX * zoom
}

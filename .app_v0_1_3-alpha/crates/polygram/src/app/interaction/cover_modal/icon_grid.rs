//! Сетка иконок с возможностью выбора.

use egui::{Color32, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use egui_lucide::Lucide;
use polygram_canvas::paint_lucide;

/// Сетка иконок с возможностью выбора.
/// Каждая иконка — уникальный вариант Lucide (все 1903 из библиотеки).
/// Возвращает `Some(rect)` выбранной иконки (координаты относительно содержимого скролла),
/// чтобы модальное окно могло прокрутить сетку к ней при открытии.
pub fn icon_grid(
    ui: &mut egui::Ui,
    search: &str,
    selected_icon: &str,
    all_icons: &[(String, Lucide)],
    new_icon: &mut Option<Lucide>,
) -> Option<Rect> {
    let container_bg = Color32::from_rgb(0x22, 0x22, 0x22);
    let container_border = Color32::from_rgb(0x37, 0x38, 0x39);
    let input_bg = Color32::from_rgb(0x25, 0x25, 0x25);

    let container_width = ui.available_width();
    let container_x = ui.cursor().min.x;

    // Фильтрация иконок по поиску (по kebab-case имени Lucide).
    let filter = search.to_lowercase();
    let filtered: Vec<&(String, Lucide)> = all_icons
        .iter()
        .filter(|(name, _)| filter.is_empty() || name.to_lowercase().contains(&filter))
        .collect();

    if filtered.is_empty() {
        ui.label("Найдено: 0");
        ui.add_space(8.0);
        return None;
    }

    const GRID_STEP_X: f32 = 60.0;
    const GRID_STEP_Y: f32 = 44.0;
    const GRID_BTN_SIZE_X: f32 = 54.0;
    const GRID_BTN_SIZE_Y: f32 = 38.0;
    const GRID_ICON_SIZE: f32 = 16.0;

    let step_x = GRID_STEP_X;
    let step_y = GRID_STEP_Y;
    let max_cols = ((container_width / step_x).floor() as usize).max(1);
    let rows = (filtered.len() + max_cols - 1) / max_cols;
    let grid_h = rows as f32 * step_y;
    let grid_start_y = ui.cursor().min.y;

    let grid_rect = Rect::from_min_size(
        Pos2::new(container_x, grid_start_y),
        Vec2::new(container_width, grid_h),
    );
    ui.painter().rect_filled(grid_rect, 2.0, container_bg);
    ui.painter().rect_stroke(
        grid_rect,
        2.0,
        Stroke::new(1.0, container_border),
        StrokeKind::Middle,
    );

    let btn_size = Vec2::new(GRID_BTN_SIZE_X, GRID_BTN_SIZE_Y);
    let icon_size = GRID_ICON_SIZE;

    // Сначала выделяем все регионы взаимодействия (уникальные ID через allocate_rect).
    let mut btn_rects: Vec<Rect> = Vec::with_capacity(filtered.len());
    for row in 0..rows {
        for col in 0..max_cols {
            let idx = row * max_cols + col;
            if idx >= filtered.len() {
                break;
            }
            let btn_x = container_x + col as f32 * step_x + (step_x - btn_size.x) / 2.0;
            let btn_y = grid_rect.min.y + row as f32 * step_y + (step_y - btn_size.y) / 2.0;
            let btn_rect = Rect::from_min_size(Pos2::new(btn_x, btn_y), btn_size);
            btn_rects.push(btn_rect);
        }
    }

    // Регистрируем все интерактивные зоны.
    let responses: Vec<egui::Response> = btn_rects
        .iter()
        .map(|rect| ui.allocate_rect(*rect, Sense::click()))
        .collect();

    // Затем рисуем все иконки.
    let mut selected_rect: Option<Rect> = None;
    for (idx, (_label, icon)) in filtered.iter().enumerate() {
        let btn_rect = btn_rects[idx];
        let response = &responses[idx];

        let is_selected = icon.name() == selected_icon;
        if is_selected {
            selected_rect = Some(btn_rect);
        }
        let hovered = response.hovered();

        let fill = if is_selected {
            Color32::from_rgb(0x34, 0x34, 0x34)
        } else if hovered {
            Color32::from_rgb(0x28, 0x28, 0x28)
        } else {
            input_bg
        };
        let border = if is_selected {
            Color32::WHITE
        } else {
            container_border
        };
        ui.painter().rect_filled(btn_rect, 2.0, fill);
        ui.painter().rect_stroke(
            btn_rect,
            2.0,
            Stroke::new(1.0, border),
            StrokeKind::Middle,
        );

        let icon_rect = Rect::from_center_size(btn_rect.center(), Vec2::splat(icon_size));
        let icon_color = if is_selected {
            Color32::WHITE
        } else {
            Color32::from_rgb(0xD4, 0xD4, 0xD8)
        };
        paint_lucide(ui, icon_rect, *icon, icon_color);

        if response.clicked() {
            *new_icon = Some(*icon);
        }
    }

    ui.advance_cursor_after_rect(Rect::from_min_size(
        Pos2::new(container_x, grid_start_y),
        Vec2::new(container_width, grid_h),
    ));
    selected_rect
}

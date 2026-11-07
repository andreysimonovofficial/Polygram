use crate::theme::Theme;
use egui::{Painter, Rect, Stroke, Vec2};
use polygram_core::{DynamicState, ObjectGeom};

pub fn paint_object(
    painter: &Painter,
    object: &ObjectGeom,
    _dynamic: &DynamicState,
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
}

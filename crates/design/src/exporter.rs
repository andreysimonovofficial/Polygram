//! Экспорт дизайна в формат, применимый в основном приложении.
//! Преобразует DesignDocument в Theme polygram-canvas и сохраняет JSON-файл,
//! который Агент может передать в Canvas для точной отрисовки.

use crate::design_format::*;
use egui::Color32;
use polygram_canvas::Theme;

/// Экспортировать дизайн в JSON-строку для Агента.
pub fn export_design(doc: &DesignDocument) -> Result<String, serde_json::Error> {
    doc.to_json()
}

/// Преобразовать DesignDocument в Theme polygram-canvas.
/// Цвета берутся из темы документа, геометрия — из переданного ObjectStyle,
/// чтобы Агент отрисовал макет в точности, как задан в инструменте дизайна.
pub fn design_to_theme(doc: &DesignDocument, style: &ObjectStyle) -> Theme {
    let t = &doc.theme;
    let shadow = style.shadow.as_ref();
    let glow = style.glow.as_ref();
    Theme {
        bg: Color32::from_rgb(t.bg.r, t.bg.g, t.bg.b),
        grid: Color32::from_rgb(t.grid.r, t.grid.g, t.grid.b),
        card_bg: Color32::from_rgb(t.card_bg.r, t.card_bg.g, t.card_bg.b),
        card_border: Color32::from_rgb(t.card_border.r, t.card_border.g, t.card_border.b),
        card_header: Color32::from_rgb(t.card_header.r, t.card_header.g, t.card_header.b),
        line: Color32::from_rgb(t.line.r, t.line.g, t.line.b),
        line_hover: Color32::from_rgb(t.line_hover.r, t.line_hover.g, t.line_hover.b),
        status_ok: Color32::from_rgb(t.status_ok.r, t.status_ok.g, t.status_ok.b),
        status_warning: Color32::from_rgb(
            t.status_warning.r,
            t.status_warning.g,
            t.status_warning.b,
        ),
        status_deficit: Color32::from_rgb(
            t.status_deficit.r,
            t.status_deficit.g,
            t.status_deficit.b,
        ),
        status_surplus: Color32::from_rgb(
            t.status_surplus.r,
            t.status_surplus.g,
            t.status_surplus.b,
        ),
        status_unconnected: Color32::from_rgb(
            t.status_unconnected.r,
            t.status_unconnected.g,
            t.status_unconnected.b,
        ),
        status_invalid: Color32::from_rgb(
            t.status_invalid.r,
            t.status_invalid.g,
            t.status_invalid.b,
        ),
        text: Color32::from_rgb(t.text.r, t.text.g, t.text.b),
        text_muted: Color32::from_rgb(t.text_muted.r, t.text_muted.g, t.text_muted.b),
        object_corner_radius: style.corner_radius,
        object_border_width: style.border_width,
        object_border_hover_width: (style.border_width * 2.0).max(1.0),
        object_header_height_cells: style.header_height_cells,
        container_header_height_cells: style.header_height_cells * 0.5,
        port_radius: style.port_radius,
        edge_width: style.border_width,
        shadow_offset: shadow.map_or([4.0, 4.0], |s| s.offset),
        shadow_blur: shadow.map_or(8.0, |s| s.blur),
        shadow_color: shadow
            .map(|s| Color32::from_rgba_unmultiplied(s.color.r, s.color.g, s.color.b, s.color.a))
            .unwrap_or(Color32::from_rgba_unmultiplied(0, 0, 0, 80)),
        shadow_enabled: shadow.is_some(),
        glow_color: glow
            .map(|g| Color32::from_rgba_unmultiplied(g.color.r, g.color.g, g.color.b, g.color.a))
            .unwrap_or(Color32::from_rgba_unmultiplied(100, 100, 200, 60)),
        glow_size: glow.map_or(6.0, |g| g.size),
        glow_enabled: glow.is_some(),
    }
}

/// Сохранить дизайн в файл.
pub fn save_to_file(
    doc: &DesignDocument,
    path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let json = doc.to_json()?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Загрузить дизайн из файла.
pub fn load_from_file(
    path: &std::path::Path,
) -> Result<DesignDocument, Box<dyn std::error::Error>> {
    let json = std::fs::read_to_string(path)?;
    let doc = DesignDocument::load_from_json(&json)?;
    Ok(doc)
}

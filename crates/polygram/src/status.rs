use crate::app::PolygramApp;

pub fn status_bar(app: &PolygramApp, ui: &mut egui::Ui) {
    ui.separator();
    let published = app.connection.snapshot.as_deref().unwrap_or_else(|| {
        // Возвращаем временный пустой Published, если снимка ещё нет.
        static EMPTY: std::sync::OnceLock<polygram_core::Published> = std::sync::OnceLock::new();
        EMPTY.get_or_init(|| polygram_core::Published {
            rev: 0,
            dynamic: polygram_core::values::DynamicState::default(),
            geometry: polygram_core::geometry::Geometry::default(),
            metric_defs: vec![],
        })
    });
    let g = &published.geometry;
    let mut text = format!(
        "Ревизия {} | Объектов: {} | Связей: {} | {} | API: {}",
        published.rev,
        g.objects.len(),
        g.edges.len(),
        app.last_status,
        if app.connection.api_connected { "онлайн" } else { "офлайн" }
    );
    if app.show_cursor_coords {
        if let Some(pos) = ui.ctx().pointer_hover_pos() {
            text.push_str(&format!(" | Координаты курсора: X: {}; Y: {}", pos.x as i32, pos.y as i32));
        }
    }
    ui.label(text);
}

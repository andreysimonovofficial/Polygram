use crate::app::PolygramApp;

pub fn status_bar(app: &PolygramApp, ui: &mut egui::Ui) {
    ui.separator();
    let published = app.snapshot.as_deref().unwrap_or_else(|| {
        // Возвращаем временный пустой Published, если снимка ещё нет.
        static EMPTY: std::sync::OnceLock<polygram_core::Published> = std::sync::OnceLock::new();
        EMPTY.get_or_init(|| polygram_core::Published {
            rev: 0,
            dynamic: polygram_core::values::DynamicState::default(),
            geometry: polygram_core::geometry::Geometry::default(),
        })
    });
    let g = &published.geometry;
    ui.label(format!(
        "Ревизия {} | Объектов: {} | Связей: {} | {} | API: {}",
        published.rev,
        g.objects.len(),
        g.edges.len(),
        app.last_status,
        if app.api_connected { "онлайн" } else { "офлайн" }
    ));
}

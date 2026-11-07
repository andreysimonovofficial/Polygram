use crate::app::PolygramApp;

pub fn status_bar(app: &PolygramApp, ui: &mut egui::Ui) {
    ui.separator();
    let published = app.runtime.handle().latest();
    let g = &published.geometry;
    ui.label(format!(
        "Ревизия {} | Объектов: {} | Связей: {} | {}",
        published.rev,
        g.objects.len(),
        g.edges.len(),
        app.last_status,
    ));
}

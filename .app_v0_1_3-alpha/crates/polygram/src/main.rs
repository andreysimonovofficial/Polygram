//! Оболочка egui Polygram: главное окно и холст.
//! Точка входа приложения: отрисовывает фон и точечную сетку холста в CentralPanel.

mod app;

use app::PolygramApp;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("Polygram — чистовик"),
        ..Default::default()
    };
    eframe::run_native(
        "Polygram",
        options,
        Box::new(|_cc| Ok(Box::new(PolygramApp::new()))),
    )
}

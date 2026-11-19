//! Оболочка egui Polygram: главное окно и холст.
//! Точка входа приложения: отрисовывает фон и точечную сетку холста в CentralPanel.

mod app;

use app::PolygramApp;

fn load_icon() -> egui::IconData {
    let img = image::load_from_memory(include_bytes!("../assets/favicon.png"))
        .expect("Failed to load favicon.png");
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    egui::IconData {
        rgba: rgba.into_raw(),
        width,
        height,
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("Polygram v0.1.3-alpha")
            .with_maximized(true)
            .with_icon(load_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "Polygram",
        options,
        Box::new(|_cc| Ok(Box::new(PolygramApp::new()))),
    )
}

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
    std::panic::set_hook(Box::new(|info| {
        use std::io::Write;
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown panic".to_string()
        };
        let location = info.location().map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column())).unwrap_or_default();
        let _ = std::fs::File::create("panic.log").and_then(|mut f| {
            f.write_all(format!("PANIC: {}\nLOCATION: {}\nBACKTRACE:\n{:?}\n", msg, location, std::backtrace::Backtrace::force_capture()).as_bytes())
        });
        eprintln!("PANIC: {msg}\nLOCATION: {location}");
    }));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("Polygram v0.1.3-alpha")
            .with_maximized(true)
            .with_active(true)
            .with_icon(load_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "Polygram",
        options,
        Box::new(|_cc| Ok(Box::new(PolygramApp::new()))),
    )
}

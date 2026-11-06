use polygram_design::DesignApp;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_title("Polygram Design Tool — Figma для Rust"),
        ..Default::default()
    };
    eframe::run_native(
        "Polygram Design Tool",
        options,
        Box::new(|_cc| Ok(Box::new(DesignApp::new()))),
    )
}

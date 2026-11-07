//! Окно Polygram: egui-оболочка с меню, палитрой команд, строкой статуса и холстом.
//! Shell владеет `EngineRuntime` и `Canvas`. Команды из UI идут в ядро,
//! снимки (`DynamicState` + `Geometry`) — на холст.

mod app;
mod interaction;
mod menu;
mod palette;
mod panels;
mod status;

use app::PolygramApp;
use eframe::egui;

fn load_icon() -> egui::IconData {
    let img = image::load_from_memory(include_bytes!("../assets/logo.png"))
        .expect("Failed to load logo.png");
    let mut rgba = img.to_rgba8();
    for pixel in rgba.pixels_mut() {
        let [r, g, b, _a] = pixel.0;
        if r > 240 && g > 240 && b > 240 {
            pixel.0 = [r, g, b, 0];
        }
    }
    let (width, height) = rgba.dimensions();
    egui::IconData {
        rgba: rgba.into_raw(),
        width,
        height,
    }
}

impl eframe::App for PolygramApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut close_menu = false;
        let published = self.runtime.handle().latest();

        menu::top_menu(self, ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            interaction::handle_canvas_interaction(self, ctx, ui, &published, &mut close_menu);
        });

        interaction::context_menu(self, ctx, &published, &mut close_menu);

        interaction::handle_rename_dialog(self, ctx);

        palette::command_palette(self, ctx);

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            status::status_bar(self, ui);
        });

        panels::command_log_panel(self, ctx);

        ctx.request_repaint();
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("Polygram — живая карта системы")
            .with_icon(load_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "Polygram Shell",
        options,
        Box::new(|_cc| Ok(Box::new(PolygramApp::new()))),
    )
}

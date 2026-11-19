//! Состояние приложения Polygram.
//! Хранит Canvas и вызывает его отрисовку в CentralPanel.

use polygram_canvas::Canvas;

/// Главное состояние приложения Polygram.
pub struct PolygramApp {
    canvas: Canvas,
}

impl PolygramApp {
    /// Создаёт новое состояние приложения с холстом по умолчанию.
    pub fn new() -> Self {
        Self {
            canvas: Canvas::new(),
        }
    }
}

impl Default for PolygramApp {
    fn default() -> Self {
        Self::new()
    }
}

impl eframe::App for PolygramApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            self.canvas.paint(ui);
        });
        ui.ctx().request_repaint();
    }
}

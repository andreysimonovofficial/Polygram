//! Состояние приложения Polygram.
//! Хранит Canvas, отслеживает правый клик по холсту и вызывает контекстное меню.

use egui::Pos2;
use polygram_canvas::Canvas;

mod icons;
mod interaction;

/// Главное состояние приложения Polygram.
pub struct PolygramApp {
    canvas: Canvas,
    /// Экранная позиция последнего правого клика по холсту (открывает контекстное меню).
    right_click: Option<Pos2>,
}

impl PolygramApp {
    /// Создаёт новое состояние приложения с холстом по умолчанию.
    pub fn new() -> Self {
        Self {
            canvas: Canvas::new(),
            right_click: None,
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

            // Фиксируем правый клик по холсте для открытия контекстного меню.
            let canvas_response = ui.interact(
                ui.clip_rect(),
                ui.scope_id().with("canvas"),
                egui::Sense::click(),
            );
            if canvas_response.secondary_clicked() {
                if let Some(pos) = canvas_response.interact_pointer_pos() {
                    self.right_click = Some(pos);
                }
            }
        });

        interaction::context_menu(self, ui.ctx());

        ui.ctx().request_repaint();
    }
}

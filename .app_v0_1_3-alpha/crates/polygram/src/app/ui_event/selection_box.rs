//! Отрисовка и завершение выделения рамкой (selection box).

use egui::{Pos2, PointerButton, Rect, Vec2};
use polygram_core::types::NodeKind;
use polygram_core::runtime::Published;
use crate::app::PolygramApp;

impl PolygramApp {
    /// Отрисовать selection box во время драга и завершить выделение при отпускании кнопки.
    pub(crate) fn handle_selection_box(
        &mut self,
        ui: &egui::Ui,
        published: &Published,
    ) {
        if self.selection_box_start.is_some() {
            if let Some(cur) = ui.input(|i| i.pointer.hover_pos()) {
                let start = self.selection_box_start.unwrap();
                let x = start.x.min(cur.x);
                let y = start.y.min(cur.y);
                let w = (cur.x - start.x).abs();
                let h = (cur.y - start.y).abs();
                if w > 4.0 && h > 4.0 {
                    let box_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h));
                    ui.painter().rect_stroke(
                        box_rect,
                        0.0,
                        egui::Stroke::new(2.0, self.canvas.theme.text),
                        egui::StrokeKind::Middle,
                    );
                }
            }
        }

        // Завершение выделения рамкой (selection box)
        if self.selection_box_start.is_some()
            && !ui.input(|i| i.pointer.button_down(PointerButton::Primary))
        {
            if let (Some(start), Some(end)) = (
                self.selection_box_start,
                ui.input(|i| i.pointer.hover_pos()),
            ) {
                let x = start.x.min(end.x);
                let y = start.y.min(end.y);
                let w = (end.x - start.x).abs();
                let h = (end.y - start.y).abs();
                self.selection_box_start = None;
                if w > 4.0 && h > 4.0 {
                    let cell_px = self.canvas.cell_px();
                    let box_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h));
                    let ctrl = ui.ctx().input(|i| i.modifiers.ctrl);
                    let mut newly_selected: Vec<String> = Vec::new();
                    for object in &self.canvas.objects {
                        let r = &object.rect;
                        let rect_px = if object.kind == NodeKind::Process {
                            let zoom = self.canvas.camera.zoom;
                            let card_w = 512.0 * zoom;
                            let card_h = polygram_canvas::paint::object::process_total_height(
                                object,
                                &published.dynamic,
                                zoom,
                                card_w,
                            );
                            Rect::from_min_size(
                                self.canvas.cell_to_screen(r.x, r.y),
                                egui::Vec2::new(card_w, card_h),
                            )
                        } else {
                            Rect::from_min_size(
                                self.canvas.cell_to_screen(r.x, r.y),
                                egui::Vec2::new(r.w as f32 * cell_px, r.h as f32 * cell_px),
                            )
                        };
                        if box_rect.intersects(rect_px) {
                            newly_selected.push(object.id.clone());
                        }
                    }
                    if ctrl {
                        for id in &newly_selected {
                            if !self.selected_objects.iter().any(|s| s == id) {
                                self.selected_objects.push(id.clone());
                            }
                        }
                    } else {
                        self.selected_objects = newly_selected;
                    }
                }
            }
        }
    }
}

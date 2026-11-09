//! Фон холста, предварительный вид фигуры при рисовании и магнитные линии выравнивания.

use crate::free_canvas::FreeCanvas;
use crate::design_format::{FreeShape, ShapeState};
use egui::{Color32, Pos2, Rect, Stroke, Vec2};

impl FreeCanvas {
    /// Фон холста (весь viewport).
    pub(crate) fn draw_background(&mut self, ui: &mut egui::Ui, canvas_rect: Rect) {
        let painter = ui.painter();
        painter.rect_filled(canvas_rect, 0.0, egui::Color32::from_rgb(
            self.doc.bg.r, self.doc.bg.g, self.doc.bg.b,
        ));
    }

    /// Магнитные красные линии выравнивания (в экранных координатах).
    pub(crate) fn draw_magnetic_lines(&mut self, ui: &mut egui::Ui, canvas_rect: Rect, magnetic_lines: &[(f32, bool)], zoom: f32, camera: Vec2) {
        if !self.magnetic {
            return;
        }
        let painter = ui.painter();
        for (coord, vertical) in magnetic_lines {
            let screen_coord = if *vertical {
                *coord * zoom + camera.x
            } else {
                *coord * zoom + camera.y
            };
            if *vertical {
                painter.line_segment(
                    [
                        Pos2::new(screen_coord, canvas_rect.min.y),
                        Pos2::new(screen_coord, canvas_rect.max.y),
                    ],
                    Stroke::new(1.0_f32 / zoom.max(0.1), Color32::from_rgb(255, 0, 0)),
                );
            } else {
                painter.line_segment(
                    [
                        Pos2::new(canvas_rect.min.x, screen_coord),
                        Pos2::new(canvas_rect.max.x, screen_coord),
                    ],
                    Stroke::new(1.0_f32 / zoom.max(0.1), Color32::from_rgb(255, 0, 0)),
                );
            }
        }
    }

    /// Предварительный вид фигуры во время рисования.
    pub(crate) fn draw_preview(&mut self, ui: &mut egui::Ui, response: &egui::Response, canvas_rect: Rect, zoom: f32) {
        let _ = canvas_rect;
        if self.tool != crate::design_format::DrawTool::Select && self.tool != crate::design_format::DrawTool::Text {
            if let (Some(start), Some(pos)) = (self.drag_start, response.hover_pos()) {
                let world_start = self.screen_to_world(start);
                let world_pos = self.screen_to_world(pos);
                let x = world_start.x.min(world_pos.x);
                let y = world_start.y.min(world_pos.y);
                let w = (world_pos.x - world_start.x).abs();
                let h = (world_pos.y - world_start.y).abs();
                if w > 2.0 && h > 2.0 {
                    let preview = FreeShape {
                        tool: self.tool,
                        x, y, w, h,
                        ..FreeShape::default()
                    };
                    let world_rect = Rect::from_min_size(
                        Pos2::new(x, y),
                        Vec2::new(w.max(1.0), h.max(1.0)),
                    );
                    let screen_rect = self.world_rect_to_screen(world_rect);
                    let painter = ui.painter();
                    self.paint_shape(painter, &preview, screen_rect, false, ShapeState::Normal, zoom);
                }
            }
        }
    }
}

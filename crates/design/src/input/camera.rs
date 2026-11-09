//! Зум колёсиком и панорамирование средней кнопкой мыши.

use crate::free_canvas::FreeCanvas;
use egui::{PointerButton, Pos2, Rect};

impl FreeCanvas {
    /// Обработка зума и панорамирования. Возвращает ничего — меняет self.camera / self.zoom.
    pub(crate) fn handle_camera(&mut self, ui: &egui::Ui, response: &egui::Response, canvas_rect: Rect) {
        let middle_down = ui.ctx().input(|i| i.pointer.button_down(PointerButton::Middle));
        let primary_down = ui.ctx().input(|i| i.pointer.button_down(PointerButton::Primary));

        // Сброс drag_start если левая кнопка мыши отпущена и нет активного перетаскивания.
        if !primary_down && !response.dragged() && !response.drag_stopped() {
            self.drag_start = None;
            self.drag_shape_pos = None;
        }

        // Приближение/отдаление колёсиком мыши к курсору.
        let scroll = ui.ctx().input(|i| i.raw_scroll_delta);
        if scroll.y.abs() > 0.001 {
            if let Some(cursor) = ui.ctx().pointer_hover_pos() {
                if canvas_rect.contains(cursor) {
                    let zoom = self.zoom.clamp(0.1, 10.0);
                    let camera = self.camera;
                    let world_cursor = (cursor - camera) / zoom;
                    let factor = (scroll.y * 0.01).exp();
                    let new_zoom = (zoom * factor).clamp(0.1, 10.0);
                    self.camera = cursor - world_cursor * new_zoom;
                    self.zoom = new_zoom;
                    ui.ctx().request_repaint();
                }
            }
        }

        // Панорамирование средней кнопкой мыши
        if response.drag_started() && middle_down {
            self.pan_start = response.interact_pointer_pos();
            self.pan_camera_start = self.camera;
        }
        if response.dragged() && middle_down {
            if let (Some(start), Some(pos)) = (self.pan_start, response.interact_pointer_pos()) {
                self.camera = self.pan_camera_start + (pos - start);
            }
            ui.ctx().request_repaint();
        }
        if response.drag_stopped() && middle_down {
            self.pan_start = None;
        }
    }

    /// Перевод экранной координаты в мировую.
    pub(crate) fn screen_to_world(&self, p: Pos2) -> Pos2 {
        let zoom = self.zoom.clamp(0.1, 10.0);
        (p - self.camera) / zoom
    }

    /// Перевод мирового прямоугольника в экранный.
    pub(crate) fn world_rect_to_screen(&self, r: Rect) -> Rect {
        let zoom = self.zoom.clamp(0.1, 10.0);
        Rect::from_min_size(r.min * zoom + self.camera, r.size() * zoom)
    }
}

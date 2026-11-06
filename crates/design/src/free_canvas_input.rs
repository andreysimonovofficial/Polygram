//! Обработка ввода на холсте: зум, панорамирование, перетаскивание, магнитное выравнивание,
//! создание фигур, предварительный просмотр.

use crate::design_format::{DrawTool, FreeShape};
use crate::free_canvas::FreeCanvas;
use egui::{PointerButton, Pos2, Rect, Sense, Stroke, Vec2};
use egui::Color32;

impl FreeCanvas {
    /// Центральная область: холст с обработкой ввода.
    pub(crate) fn canvas_panel(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size();
        let canvas_rect = egui::Rect::from_min_size(ui.cursor().min, available);

        // Обработка ввода сначала (избегаем конфликта заимствований с painter)
        let response = ui.allocate_rect(canvas_rect, Sense::click_and_drag());
        let painter = ui.painter();

        let middle_down = ui.ctx().input(|i| i.pointer.button_down(PointerButton::Middle));

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
        }
        if response.drag_stopped() && middle_down {
            self.pan_start = None;
        }

        let camera = self.camera;
        let zoom = self.zoom.clamp(0.1, 10.0);

        // Вспомогательные замыкания для перевода координат
        let screen_to_world = |p: Pos2| (p - camera) / zoom;
        let world_rect_to_screen = |r: Rect| Rect::from_min_size(r.min * zoom + camera, r.size() * zoom);

        // Фон холста (весь viewport)
        painter.rect_filled(canvas_rect, 0.0, egui::Color32::from_rgb(
            self.doc.bg.r, self.doc.bg.g, self.doc.bg.b,
        ));

        // Магнитные линии (отображаются поверх всего в конце)
        let mut magnetic_lines: Vec<(f32, bool)> = Vec::new(); // (мировая координата, вертикальная)

        // Отрисовать все фигуры
        for (i, shape) in self.doc.shapes.iter().enumerate() {
            let is_selected = self.selected == Some(i);
            let world_rect = Rect::from_min_size(
                Pos2::new(shape.x, shape.y),
                Vec2::new(shape.w.max(1.0), shape.h.max(1.0)),
            );
            let screen_rect = world_rect_to_screen(world_rect);
            let screen_clip = screen_rect.intersect(canvas_rect);
            if screen_clip.is_positive() {
                self.paint_shape(painter, shape, screen_rect, is_selected);
            }
        }

        // Выделение
        if let Some(idx) = self.selected {
            if let Some(shape) = self.doc.shapes.get(idx) {
                let world_rect = Rect::from_min_size(
                    Pos2::new(shape.x, shape.y),
                    Vec2::new(shape.w.max(1.0), shape.h.max(1.0)),
                );
                let screen_rect = world_rect_to_screen(world_rect);
                painter.rect_stroke(
                    screen_rect.expand(3.0),
                    0.0,
                    Stroke::new(1.0_f32, Color32::from_rgb(100, 180, 255)),
                );
            }
        }

        if response.hovered() {
            match self.tool {
                DrawTool::Select => ui.ctx().set_cursor_icon(egui::CursorIcon::Default),
                _ => ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair),
            }
        }

        // Нажатие мыши — для инструмента Текст размещаем текстовую фигуру по клику
        if response.clicked() && !middle_down {
            if let Some(pos) = response.interact_pointer_pos() {
                if self.tool == DrawTool::Text {
                    let mut shape = FreeShape::default();
                    shape.id = format!("shape-{}", self.next_id);
                    self.next_id += 1;
                    shape.tool = DrawTool::Text;
                    let wp = screen_to_world(pos);
                    shape.x = wp.x;
                    shape.y = wp.y;
                    shape.w = 140.0 / zoom;
                    shape.h = 30.0 / zoom;
                    shape.text = "Текст".to_owned();
                    self.doc.shapes.push(shape);
                    self.selected = Some(self.doc.shapes.len() - 1);
                    self.drag_start = None;
                    self.drag_shape_pos = None;
                    self.touch();
                } else if self.tool == DrawTool::Select {
                    let world_pos = screen_to_world(pos);
                    self.selected = self.hit_shape(world_pos);
                    if let Some(idx) = self.selected {
                        if let Some(shape) = self.doc.shapes.get(idx) {
                            self.drag_shape_pos = Some((idx, Pos2::new(shape.x, shape.y)));
                        }
                    } else {
                        self.drag_shape_pos = None;
                    }
                    self.drag_start = response.interact_pointer_pos();
                }
            }
        }

        // Начало перетаскивания / рисования (только левая кнопка)
        if response.drag_started() && !middle_down {
            self.drag_start = response.interact_pointer_pos();
            if self.tool == DrawTool::Select {
                if let Some(pos) = self.drag_start {
                    let world_pos = screen_to_world(pos);
                    self.selected = self.hit_shape(world_pos);
                    if let Some(idx) = self.selected {
                        if let Some(shape) = self.doc.shapes.get(idx) {
                            self.drag_shape_pos =
                                Some((idx, Pos2::new(shape.x, shape.y)));
                        }
                    } else {
                        self.drag_shape_pos = None;
                    }
                }
            } else {
                self.drag_shape_pos = None;
            }
        }

        // Перетаскивание выделенной фигуры
        if response.dragged() && !middle_down && self.tool == DrawTool::Select {
            if let Some(pos) = response.interact_pointer_pos() {
                if let Some((idx, origin)) = self.drag_shape_pos {
                    let world_cursor = screen_to_world(pos);
                    let drag_world_start = screen_to_world(self.drag_start.unwrap_or(pos));
                    let mut nx = origin.x + (world_cursor.x - drag_world_start.x);
                    let mut ny = origin.y + (world_cursor.y - drag_world_start.y);

                    if self.magnetic {
                        if let Some(shape) = self.doc.shapes.get(idx) {
                            let w = shape.w.max(1.0);
                            let h = shape.h.max(1.0);
                            let thresh = 6.0;
                            // Магнитится к краям и центрам других фигур
                            for (j, other) in self.doc.shapes.iter().enumerate() {
                                if j == idx {
                                    continue;
                                }
                                let or = Rect::from_min_size(
                                    Pos2::new(other.x, other.y),
                                    Vec2::new(other.w.max(1.0), other.h.max(1.0)),
                                );
                                // Края по X: левый/правый/центр
                                for ox in [or.min.x, or.max.x, or.center().x] {
                                    if (nx - ox).abs() < thresh {
                                        nx = ox;
                                        magnetic_lines.push((ox, true));
                                    }
                                    if ((nx + w) - ox).abs() < thresh {
                                        nx = ox - w;
                                        magnetic_lines.push((ox, true));
                                    }
                                }
                                // Края по Y: верхний/нижний/центр
                                for oy in [or.min.y, or.max.y, or.center().y] {
                                    if (ny - oy).abs() < thresh {
                                        ny = oy;
                                        magnetic_lines.push((oy, false));
                                    }
                                    if ((ny + h) - oy).abs() < thresh {
                                        ny = oy - h;
                                        magnetic_lines.push((oy, false));
                                    }
                                }
                            }
                            // Центр объекта к центру объекта
                            let dragged_center_x = nx + w / 2.0;
                            let dragged_center_y = ny + h / 2.0;
                            for other in &self.doc.shapes {
                                let ocx = other.x + other.w.max(1.0) / 2.0;
                                let ocy = other.y + other.h.max(1.0) / 2.0;
                                if (dragged_center_x - ocx).abs() < thresh {
                                    nx = ocx - w / 2.0;
                                    magnetic_lines.push((ocx, true));
                                }
                                if (dragged_center_y - ocy).abs() < thresh {
                                    ny = ocy - h / 2.0;
                                    magnetic_lines.push((ocy, false));
                                }
                            }
                        }
                    }

                    if let Some(shape) = self.doc.shapes.get_mut(idx) {
                        shape.x = nx;
                        shape.y = ny;
                        self.touch();
                    }
                }
            }
        }

        // Завершение перетаскивания — создать новую фигуру (кроме Select и Text)
        if response.drag_stopped() && !middle_down {
            if self.tool != DrawTool::Select && self.tool != DrawTool::Text {
                if let (Some(start), Some(end)) = (self.drag_start, response.interact_pointer_pos()) {
                    let world_start = screen_to_world(start);
                    let world_end = screen_to_world(end);
                    let mut x = world_start.x.min(world_end.x);
                    let mut y = world_start.y.min(world_end.y);
                    let w = (world_end.x - world_start.x).abs();
                    let h = (world_end.y - world_start.y).abs();

                    if self.magnetic && w > 4.0 && h > 4.0 {
                        for other in &self.doc.shapes {
                            let or = Rect::from_min_size(
                                Pos2::new(other.x, other.y),
                                Vec2::new(other.w.max(1.0), other.h.max(1.0)),
                            );
                            let thresh = 6.0;
                            for ox in [or.min.x, or.max.x, or.center().x] {
                                if (x - ox).abs() < thresh {
                                    x = ox;
                                    magnetic_lines.push((ox, true));
                                }
                                if ((x + w) - ox).abs() < thresh {
                                    x = ox - w;
                                    magnetic_lines.push((ox, true));
                                }
                            }
                            for oy in [or.min.y, or.max.y, or.center().y] {
                                if (y - oy).abs() < thresh {
                                    y = oy;
                                    magnetic_lines.push((oy, false));
                                }
                                if ((y + h) - oy).abs() < thresh {
                                    y = oy - h;
                                    magnetic_lines.push((oy, false));
                                }
                            }
                        }
                    }

                    if w > 4.0 && h > 4.0 {
                        let mut shape = FreeShape::default();
                        shape.id = format!("shape-{}", self.next_id);
                        self.next_id += 1;
                        shape.tool = self.tool;
                        shape.x = x;
                        shape.y = y;
                        shape.w = w;
                        shape.h = h;
                        self.doc.shapes.push(shape);
                        self.selected = Some(self.doc.shapes.len() - 1);
                        self.touch();
                    }
                }
            }
            self.drag_start = None;
            self.drag_shape_pos = None;
        }

        // Магнитные красные линии выравнивания (в экранных координатах)
        if self.magnetic {
            for (coord, vertical) in &magnetic_lines {
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

        // Предварительный вид фигуры во время рисования
        if self.tool != DrawTool::Select && self.tool != DrawTool::Text {
            if let (Some(start), Some(pos)) = (self.drag_start, response.hover_pos()) {
                let world_start = screen_to_world(start);
                let world_pos = screen_to_world(pos);
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
                    let screen_rect = world_rect_to_screen(world_rect);
                    self.paint_shape(painter, &preview, screen_rect, false);
                }
            }
        }

        ui.allocate_rect(canvas_rect, Sense::hover());
    }
}

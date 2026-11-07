//! Обработка ввода на холсте: зум, панорамирование, перетаскивание, магнитное выравнивание,
//! создание фигур, предварительный просмотр, контекстное меню и группировка.

use crate::design_format::{DrawTool, FreeShape, ShapeState, ChildReactMode};
use crate::free_canvas::FreeCanvas;
use egui::{PointerButton, Pos2, Rect, Sense, Stroke, Vec2};
use egui::Color32;
use std::collections::HashSet;

/// Режим изменения размера фигуры за угол или сторону.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResizeMode {
    NW, N, NE,
    W,     E,
    SW, S, SE,
}

impl FreeCanvas {
    /// Центральная область: холст с обработкой ввода.
    pub(crate) fn canvas_panel(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size();
        let canvas_rect = egui::Rect::from_min_size(ui.cursor().min, available);

        // Обработка ввода сначала (избегаем конфликта заимствований с painter)
        let response = ui.allocate_rect(canvas_rect, Sense::click_and_drag());
        let painter = ui.painter();

        let middle_down = ui.ctx().input(|i| i.pointer.button_down(PointerButton::Middle));
        let primary_down = ui.ctx().input(|i| i.pointer.button_down(PointerButton::Primary));

        // Сброс drag_start если левая кнопка мыши отпущена и нет активного перетаскивания.
        // Защита от "залипания" состояния, из-за которого preview фигуры рисуется без зажатия мыши.
        // Не сбрасываем во время drag_stopped, чтобы фигура успела создатьс€.
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

        // Отслеживание наведения и нажатия в режиме прототипа.
        if self.prototype_mode {
            if let Some(cursor) = ui.ctx().pointer_hover_pos() {
                if canvas_rect.contains(cursor) {
                    let world_cursor = screen_to_world(cursor);
                    let direct = self.direct_hit(world_cursor);
                    self.direct_hover = direct;
                    self.hovered = self.active_set(direct);
                } else {
                    self.direct_hover = None;
                    self.hovered.clear();
                }
            } else {
                self.direct_hover = None;
                self.hovered.clear();
            }
            let mouse_down = ui.ctx().input(|i| i.pointer.button_down(PointerButton::Primary));
            if mouse_down {
                if self.pressed.is_empty() {
                    if let Some(direct) = self.direct_hover {
                        self.direct_press = Some(direct);
                        self.pressed = self.active_set(Some(direct));
                    }
                }
            } else if !self.pressed.is_empty() {
                self.direct_press = None;
                self.pressed.clear();
            }
        } else {
            self.direct_hover = None;
            self.direct_press = None;
            self.hovered.clear();
            self.pressed.clear();
        }

        // Магнитные линии (отображаются поверх всего в конце)
        let mut magnetic_lines: Vec<(f32, bool)> = Vec::new(); // (мировая координата, вертикальная)

        // Отрисовать все фигуры
        for (i, shape) in self.doc.shapes.iter().enumerate() {
            let is_selected = self.selected.contains(&i);
            let world_rect = Rect::from_min_size(
                Pos2::new(shape.x, shape.y),
                Vec2::new(shape.w.max(1.0), shape.h.max(1.0)),
            );
            let screen_rect = world_rect_to_screen(world_rect);
            let screen_clip = screen_rect.intersect(canvas_rect);
            if screen_clip.is_positive() {
                let state = if self.prototype_mode {
                    Self::shape_state(&self.hovered, &self.pressed, i)
                } else {
                    ShapeState::Normal
                };
                self.paint_shape(painter, shape, screen_rect, is_selected, state, zoom);
            }
        }

        if response.hovered() && self.resize_mode.is_none() {
            match self.tool {
                DrawTool::Select => ui.ctx().set_cursor_icon(egui::CursorIcon::Default),
                _ => ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair),
            }
        }

        // Единый фрейм выделения вокруг всех выделенных фигур
        let mut screen_rect: Option<Rect> = None;
        if !self.selected.is_empty() {
            let mut min_x = f32::INFINITY;
            let mut min_y = f32::INFINITY;
            let mut max_x = f32::NEG_INFINITY;
            let mut max_y = f32::NEG_INFINITY;
            for &idx in &self.selected {
                if let Some(shape) = self.doc.shapes.get(idx) {
                    min_x = min_x.min(shape.x);
                    min_y = min_y.min(shape.y);
                    max_x = max_x.max(shape.x + shape.w);
                    max_y = max_y.max(shape.y + shape.h);
                }
            }
            if min_x.is_finite() {
                let world_rect = Rect::from_min_size(
                    Pos2::new(min_x, min_y),
                    Vec2::new(max_x - min_x, max_y - min_y),
                );
                screen_rect = Some(world_rect_to_screen(world_rect));
                let sr = screen_rect.unwrap();
                painter.rect_stroke(
                    sr.expand(4.0),
                    0.0,
                    Stroke::new(1.5_f32 * zoom, Color32::from_rgb(100, 180, 255)),
                );

                // Ручки изменения размера (углы и стороны выделения).
                    if !self.prototype_mode {
                    let handle_size = 12.0;
                    let hit_size = 16.0;
                    let hover_pos = ui.ctx().pointer_hover_pos();
                    let mut hovered_handle: Option<ResizeMode> = None;

                    let handles: [(ResizeMode, Pos2); 8] = [
                        (ResizeMode::NW, Pos2::new(sr.min.x, sr.min.y)),
                        (ResizeMode::N, Pos2::new(sr.center().x, sr.min.y)),
                        (ResizeMode::NE, Pos2::new(sr.max.x, sr.min.y)),
                        (ResizeMode::W, Pos2::new(sr.min.x, sr.center().y)),
                        (ResizeMode::E, Pos2::new(sr.max.x, sr.center().y)),
                        (ResizeMode::SW, Pos2::new(sr.min.x, sr.max.y)),
                        (ResizeMode::S, Pos2::new(sr.center().x, sr.max.y)),
                        (ResizeMode::SE, Pos2::new(sr.max.x, sr.max.y)),
                    ];

                    for (mode, center) in handles {
                        let handle_rect = Rect::from_center_size(center, Vec2::splat(handle_size));
                        let hit_rect = Rect::from_center_size(center, Vec2::splat(hit_size));
                        let is_hovered = hover_pos.map_or(false, |p| hit_rect.contains(p));
                        if is_hovered {
                            hovered_handle = Some(mode);
                        }
                        painter.rect_filled(handle_rect, 2.0, Color32::from_rgb(100, 180, 255));
                        painter.rect_stroke(handle_rect, 2.0, Stroke::new(1.0_f32, Color32::from_rgb(255, 255, 255)));
                    }

                    // Курсор для ручек ресайза.
                    if let Some(mode) = hovered_handle {
                    let cursor = match mode {
                        ResizeMode::NW | ResizeMode::SE => egui::CursorIcon::ResizeNorthWest,
                        ResizeMode::NE | ResizeMode::SW => egui::CursorIcon::ResizeNorthEast,
                        ResizeMode::N | ResizeMode::S => egui::CursorIcon::ResizeRow,
                        ResizeMode::W | ResizeMode::E => egui::CursorIcon::ResizeColumn,
                    };
                        ui.ctx().set_cursor_icon(cursor);
                    }
                }
            }
        }

        // Контекстное меню правой кнопкой мыши
        if response.secondary_clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let world_pos = screen_to_world(pos);
                if let Some(idx) = self.hit_shape(world_pos) {
                    if !self.selected.contains(&idx) {
                        self.selected = vec![idx];
                    }
                    self.context_menu = Some((pos, Some(idx)));
                    self.touch();
                } else {
                    self.selected.clear();
                    self.context_menu = Some((pos, None));
                    self.touch();
                }
            }
        }

        // Отрисовать контекстное меню если нужно
        if let Some((menu_pos, target_idx)) = self.context_menu.clone() {
            self.show_context_menu(ui.ctx(), menu_pos, target_idx);
        }

        // Закрыть контекстное меню при левом клике вне меню
        let menu_was_open = self.context_menu.is_some();
        if response.clicked() && menu_was_open {
            self.context_menu = None;
        }

        // Нажатие мыши — для инструмента Текст размещаем текстовую фигуру по клику
        if response.clicked() && !middle_down && !menu_was_open {
            if let Some(pos) = response.interact_pointer_pos() {
                if self.tool == DrawTool::Text {
                    self.save_undo_state();
                    let mut shape = FreeShape::default();
                    shape.id = format!("shape-{}", self.next_id);
                    self.next_id += 1;
                    shape.tool = DrawTool::Text;
                    let wp = screen_to_world(pos);
                    shape.x = wp.x;
                    shape.y = wp.y;
                    shape.w = 140.0;
                    shape.h = 30.0;
                    shape.fill = None;
                    shape.stroke_width = 0.0;
                    shape.text = "Текст".to_owned();
                    self.doc.shapes.push(shape);
                    self.selected = vec![self.doc.shapes.len() - 1];
                    self.drag_start = None;
                    self.drag_shape_pos = None;
                    self.touch();
                } else if self.tool == DrawTool::Select {
                    // Пропустить клик, если попали в ручку ресайза.
                    let over_resize = if !self.selected.is_empty() && !self.prototype_mode {
                        let mut min_x = f32::INFINITY;
                        let mut min_y = f32::INFINITY;
                        let mut max_x = f32::NEG_INFINITY;
                        let mut max_y = f32::NEG_INFINITY;
                        for &idx in &self.selected {
                            if let Some(shape) = self.doc.shapes.get(idx) {
                                min_x = min_x.min(shape.x);
                                min_y = min_y.min(shape.y);
                                max_x = max_x.max(shape.x + shape.w);
                                max_y = max_y.max(shape.y + shape.h);
                            }
                        }
                        if min_x.is_finite() {
                            let world_rect = Rect::from_min_size(
                                Pos2::new(min_x, min_y),
                                Vec2::new(max_x - min_x, max_y - min_y),
                            );
                            let sr = world_rect_to_screen(world_rect);
                            let handle_size = 9.0;
                            let handles: [(ResizeMode, Pos2); 8] = [
                                (ResizeMode::NW, Pos2::new(sr.min.x, sr.min.y)),
                                (ResizeMode::N, Pos2::new(sr.center().x, sr.min.y)),
                                (ResizeMode::NE, Pos2::new(sr.max.x, sr.min.y)),
                                (ResizeMode::W, Pos2::new(sr.min.x, sr.center().y)),
                                (ResizeMode::E, Pos2::new(sr.max.x, sr.center().y)),
                                (ResizeMode::SW, Pos2::new(sr.min.x, sr.max.y)),
                                (ResizeMode::S, Pos2::new(sr.center().x, sr.max.y)),
                                (ResizeMode::SE, Pos2::new(sr.max.x, sr.max.y)),
                            ];
                            handles.iter().any(|(_, center)| {
                                Rect::from_center_size(*center, Vec2::splat(handle_size)).contains(pos)
                            })
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    if !over_resize {
                        let world_pos = screen_to_world(pos);
                        let hit = self.hit_shape(world_pos);
                        let ctrl = ui.ctx().input(|i| i.modifiers.ctrl);

                        match hit {
                            Some(idx) => {
                                if ctrl {
                                    if self.selected.contains(&idx) {
                                        self.selected.retain(|&s| s != idx);
                                    } else {
                                        self.selected.push(idx);
                                    }
                                } else {
                                    self.selected = vec![idx];
                                }
                                if let Some(shape) = self.doc.shapes.get(idx) {
                                    self.drag_shape_pos =
                                        Some((idx, Pos2::new(shape.x, shape.y)));
                                }
                            }
                            None => {
                                if !ctrl {
                                    self.selected.clear();
                                }
                                self.drag_shape_pos = None;
                            }
                        }
                    }
                    self.drag_start = response.interact_pointer_pos();
                }
            }
        }

        // Начало перетаскивания / рисования (только левая кнопка)
        if response.drag_started() && !middle_down {
            self.drag_start = response.interact_pointer_pos();

            // Проверка попадания в ручку ресайза перед обычным перетаскиванием.
            if !self.selected.is_empty()
                && !self.prototype_mode
                && self.resize_mode.is_none()
            {
                if let Some(pos) = self.drag_start {
                    let mut min_x = f32::INFINITY;
                    let mut min_y = f32::INFINITY;
                    let mut max_x = f32::NEG_INFINITY;
                    let mut max_y = f32::NEG_INFINITY;
                    for &idx in &self.selected {
                        if let Some(shape) = self.doc.shapes.get(idx) {
                            min_x = min_x.min(shape.x);
                            min_y = min_y.min(shape.y);
                            max_x = max_x.max(shape.x + shape.w);
                            max_y = max_y.max(shape.y + shape.h);
                        }
                    }
                    if min_x.is_finite() {
                        let world_rect = Rect::from_min_size(
                            Pos2::new(min_x, min_y),
                            Vec2::new(max_x - min_x, max_y - min_y),
                        );
                        let sr = world_rect_to_screen(world_rect);
                        let hit_size = 16.0;
                        let hover_pos = pos;

                        let handles: [(ResizeMode, Pos2); 8] = [
                            (ResizeMode::NW, Pos2::new(sr.min.x, sr.min.y)),
                            (ResizeMode::N, Pos2::new(sr.center().x, sr.min.y)),
                            (ResizeMode::NE, Pos2::new(sr.max.x, sr.min.y)),
                            (ResizeMode::W, Pos2::new(sr.min.x, sr.center().y)),
                            (ResizeMode::E, Pos2::new(sr.max.x, sr.center().y)),
                            (ResizeMode::SW, Pos2::new(sr.min.x, sr.max.y)),
                            (ResizeMode::S, Pos2::new(sr.center().x, sr.max.y)),
                            (ResizeMode::SE, Pos2::new(sr.max.x, sr.max.y)),
                        ];

                        for (mode, center) in handles {
                            let handle_rect =
                                Rect::from_center_size(center, Vec2::splat(hit_size));
                            if handle_rect.contains(hover_pos) {
                                self.resize_mode = Some(mode);
                                self.drag_shape_pos = None;
                                self.resize_origin.clear();
                                self.resize_mag_locked_x = None;
                                self.resize_mag_locked_y = None;
                                for &idx in &self.selected {
                                    if let Some(shape) = self.doc.shapes.get(idx) {
                                        self.resize_origin.push((idx, shape.x, shape.y, shape.w, shape.h));
                                    }
                                }
                                self.save_undo_state();
                                self.drag_history_saved = true;
                                break;
                            }
                        }
                    }
                }
            }

            if self.resize_mode.is_none() && self.tool == DrawTool::Select {
                if let Some(pos) = self.drag_start {
                    let world_pos = screen_to_world(pos);
                    let hit = self.hit_shape(world_pos);
                    let ctrl = ui.ctx().input(|i| i.modifiers.ctrl);

                    match hit {
                        Some(idx) => {
                            if ctrl {
                                if self.selected.contains(&idx) {
                                    self.selected.retain(|&s| s != idx);
                                } else {
                                    self.selected.push(idx);
                                }
                            } else {
                                self.selected = vec![idx];
                            }
                            if let Some(shape) = self.doc.shapes.get(idx) {
                                self.drag_shape_pos =
                                    Some((idx, Pos2::new(shape.x, shape.y)));
                            }
                            self.drag_history_saved = false;
                        }
                        None => {
                            if !ctrl {
                                self.selected.clear();
                            }
                            self.drag_shape_pos = None;
                        }
                    }
                }
            } else if self.resize_mode.is_none() {
                self.drag_shape_pos = None;
            }
        }

        // Перетаскивание выделенных фигур
        if response.dragged() && !middle_down && self.tool == DrawTool::Select && self.resize_mode.is_none() {
            if !self.drag_history_saved {
                self.save_undo_state();
                self.drag_history_saved = true;
            }
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
                            for (j, other) in self.doc.shapes.iter().enumerate() {
                                if self.selected.contains(&j) {
                                    continue;
                                }
                                let or = Rect::from_min_size(
                                    Pos2::new(other.x, other.y),
                                    Vec2::new(other.w.max(1.0), other.h.max(1.0)),
                                );
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

                    for &idx in &self.selected {
                        if let Some(shape) = self.doc.shapes.get_mut(idx) {
                            shape.x = nx;
                            shape.y = ny;
                        }
                    }
                    self.touch();
                } else {
                    // Selection box (резиновый прямоугольник) при перетаскивании из пустого места.
                    let world_start = screen_to_world(self.drag_start.unwrap_or(pos));
                    let world_cursor = screen_to_world(pos);
                    let x = world_start.x.min(world_cursor.x);
                    let y = world_start.y.min(world_cursor.y);
                    let w = (world_cursor.x - world_start.x).abs();
                    let h = (world_cursor.y - world_start.y).abs();
                    if w > 1.0 && h > 1.0 {
                        let world_rect = Rect::from_min_size(
                            Pos2::new(x, y),
                            Vec2::new(w.max(1.0), h.max(1.0)),
                        );
                        let screen_rect = world_rect_to_screen(world_rect);
                        painter.rect_stroke(
                            screen_rect,
                            0.0,
                            Stroke::new(2.0_f32, Color32::from_rgb(0, 217, 255)),
                        );
                    }
                }
            }
        }

        // Изменение размера выделенных фигур за угол или сторону.
        if response.dragged() && !middle_down && self.resize_mode.is_some() {
            let mode = self.resize_mode.unwrap();
            if let Some(pos) = response.interact_pointer_pos() {
                let world_cursor = screen_to_world(pos);
                let drag_world_start = screen_to_world(self.drag_start.unwrap_or(pos));
                let dx = world_cursor.x - drag_world_start.x;
                let dy = world_cursor.y - drag_world_start.y;

                for &idx in &self.selected {
                    // Берём исходные координаты из resize_origin, а не из текущего состояния фигуры,
                    // чтобы избежать накопления «уплывания» размера между кадрами.
                    let (_orig_idx, sx, sy, sw, sh) = self
                        .resize_origin
                        .iter()
                        .find(|(i, _, _, _, _)| *i == idx)
                        .copied()
                        .unwrap_or((idx, 0.0, 0.0, 1.0, 1.0));

                    // Собрать данные других фигур для магнита.
                    let others: Vec<(Rect, f32, f32)> = if self.magnetic {
                        self.doc
                            .shapes
                            .iter()
                            .enumerate()
                            .filter(|(j, _)| !self.selected.contains(j))
                            .map(|(_, other)| {
                                let r = Rect::from_min_size(
                                    Pos2::new(other.x, other.y),
                                    Vec2::new(other.w.max(1.0), other.h.max(1.0)),
                                );
                                let cx = other.x + other.w.max(1.0) / 2.0;
                                let cy = other.y + other.h.max(1.0) / 2.0;
                                (r, cx, cy)
                            })
                            .collect()
                    } else {
                        Vec::new()
                    };

                    let mut nx = sx;
                    let mut ny = sy;
                    let mut nw = sw;
                    let mut nh = sh;

                    // Каждая фигура использует СВОИ исходные координаты.
                    match mode {
                        ResizeMode::SE => {
                            nw = (sw + dx).max(1.0);
                            nh = (sh + dy).max(1.0);
                        }
                        ResizeMode::SW => {
                            nw = (sw - dx).max(1.0);
                            nx = sx + (sw - nw);
                            nh = (sh + dy).max(1.0);
                        }
                        ResizeMode::NE => {
                            nw = (sw + dx).max(1.0);
                            nh = (sh - dy).max(1.0);
                            ny = sy + (sh - nh);
                        }
                        ResizeMode::NW => {
                            nw = (sw - dx).max(1.0);
                            nx = sx + (sw - nw);
                            nh = (sh - dy).max(1.0);
                            ny = sy + (sh - nh);
                        }
                        ResizeMode::E => {
                            nw = (sw + dx).max(1.0);
                        }
                        ResizeMode::W => {
                            nw = (sw - dx).max(1.0);
                            nx = sx + (sw - nw);
                        }
                        ResizeMode::S => {
                            nh = (sh + dy).max(1.0);
                        }
                        ResizeMode::N => {
                            nh = (sh - dy).max(1.0);
                            ny = sy + (sh - nh);
                        }
                    }

                    // Магнитное выравнивание при ресайзе (точное, без aproximативности).
                    // Гистерезис ("липкое" сцепление): после попадания в порог край удерживается
                    // ровно на цели, пока курсор не уйдёт далеко — это убирает дребезг на границе
                    // порога (форма не пляшет вверх-вниз на несколько пкс) и делает сцепление
                    // воспроизводимым при повторном захвате ручки.
                    if self.magnetic {
                        let w = nw.max(1.0);
                        let h = nh.max(1.0);
                        let thresh = 6.0;
                        let unlock_thresh = thresh * 2.0;

                        let move_top = matches!(mode, ResizeMode::N | ResizeMode::NW | ResizeMode::NE);
                        let move_bottom = matches!(mode, ResizeMode::S | ResizeMode::SW | ResizeMode::SE);
                        let move_left = matches!(mode, ResizeMode::W | ResizeMode::NW | ResizeMode::SW);
                        let move_right = matches!(mode, ResizeMode::E | ResizeMode::NE | ResizeMode::SE);

                        // Текущие (от курсора) значения подвижных краёв, до любого сцепления.
                        let raw_top = ny;
                        let raw_bottom = ny + h;
                        let raw_left = nx;
                        let raw_right = nx + w;

                        // ----- Вертикальная ось (верхний / нижний край) -----
                        let mut target_y: Option<f32> = None;
                        if move_top || move_bottom {
                            let edge_y = if move_top { raw_top } else { raw_bottom };
                            for (or, _, _) in &others {
                                for oy in [or.min.y, or.max.y, or.center().y] {
                                    if (edge_y - oy).abs() < thresh {
                                        let d = (edge_y - oy).abs();
                                        let better = target_y.map_or(true, |cur| d < (edge_y - cur).abs());
                                        if better {
                                            target_y = Some(oy);
                                        }
                                    }
                                }
                            }
                        }
                        let mut locked_y = self.resize_mag_locked_y;
                        if let Some(ty) = target_y {
                            locked_y = Some(ty);
                        } else if let Some(ly) = locked_y {
                            let edge_y = if move_top { raw_top } else { raw_bottom };
                            if (edge_y - ly).abs() > unlock_thresh {
                                locked_y = None;
                            }
                        }
                        self.resize_mag_locked_y = locked_y;
                        if let Some(ty) = locked_y {
                            magnetic_lines.push((ty, false));
                            if move_top {
                                ny = ty;
                                nh = (sy + sh) - ny;
                                nh = nh.max(1.0);
                            } else if move_bottom {
                                nh = (ty - ny).max(1.0);
                            }
                        }

                        // ----- Горизонтальная ось (левый / правый край) -----
                        let mut target_x: Option<f32> = None;
                        if move_left || move_right {
                            let edge_x = if move_left { raw_left } else { raw_right };
                            for (or, _, _) in &others {
                                for ox in [or.min.x, or.max.x, or.center().x] {
                                    if (edge_x - ox).abs() < thresh {
                                        let d = (edge_x - ox).abs();
                                        let better = target_x.map_or(true, |cur| d < (edge_x - cur).abs());
                                        if better {
                                            target_x = Some(ox);
                                        }
                                    }
                                }
                            }
                        }
                        let mut locked_x = self.resize_mag_locked_x;
                        if let Some(tx) = target_x {
                            locked_x = Some(tx);
                        } else if let Some(lx) = locked_x {
                            let edge_x = if move_left { raw_left } else { raw_right };
                            if (edge_x - lx).abs() > unlock_thresh {
                                locked_x = None;
                            }
                        }
                        self.resize_mag_locked_x = locked_x;
                        if let Some(tx) = locked_x {
                            magnetic_lines.push((tx, true));
                            if move_left {
                                nx = tx;
                                nw = (sx + sw) - nx;
                                nw = nw.max(1.0);
                            } else if move_right {
                                nw = (tx - nx).max(1.0);
                            }
                        }

                        // Центры всех фигур тоже magnets — но не Überschриваем только что зафиксированный край.
                        for (_, ocx, ocy) in &others {
                            if locked_x.is_none() && (nx + w / 2.0 - ocx).abs() < thresh {
                                nx = ocx - w / 2.0;
                                magnetic_lines.push((*ocx, true));
                            }
                            if locked_y.is_none() && (ny + h / 2.0 - ocy).abs() < thresh {
                                ny = ocy - h / 2.0;
                                magnetic_lines.push((*ocy, false));
                            }
                        }
                    }

                    if let Some(shape) = self.doc.shapes.get_mut(idx) {
                        shape.x = nx;
                        shape.y = ny;
                        shape.w = nw;
                        shape.h = nh;
                    }
                }
                self.touch();
            }
        }

        // Завершение перетаскивания — создать новую фигуру или выделить по box
        if response.drag_stopped() && !middle_down {
            let was_resizing = self.resize_mode.is_some();
            self.resize_mode = None;
            self.resize_origin.clear();
            self.resize_mag_locked_x = None;
            self.resize_mag_locked_y = None;

            if !was_resizing && self.tool == DrawTool::Select && self.drag_shape_pos.is_none() {
                // Selection box: выделить все фигуры внутри резинового прямоугольника
                if let (Some(start), Some(end)) = (self.drag_start, response.interact_pointer_pos()) {
                    let world_start = screen_to_world(start);
                    let world_end = screen_to_world(end);
                    let x = world_start.x.min(world_end.x);
                    let y = world_start.y.min(world_end.y);
                    let w = (world_end.x - world_start.x).abs();
                    let h = (world_end.y - world_start.y).abs();
                    if w > 4.0 && h > 4.0 {
                        self.save_undo_state();
                        let sel_rect = Rect::from_min_size(
                            Pos2::new(x, y),
                            Vec2::new(w.max(1.0), h.max(1.0)),
                        );
                        let mut newly_selected: Vec<usize> = Vec::new();
                        for (i, shape) in self.doc.shapes.iter().enumerate() {
                            let shape_rect = Rect::from_min_size(
                                Pos2::new(shape.x, shape.y),
                                Vec2::new(shape.w.max(1.0), shape.h.max(1.0)),
                            );
                            if sel_rect.intersects(shape_rect) {
                                newly_selected.push(i);
                            }
                        }
                        let ctrl = ui.ctx().input(|i| i.modifiers.ctrl);
                        if ctrl {
                            for &idx in &newly_selected {
                                if !self.selected.contains(&idx) {
                                    self.selected.push(idx);
                                }
                            }
                        } else {
                            self.selected = newly_selected;
                        }
                        self.touch();
                    }
                }
            } else if self.tool != DrawTool::Select && self.tool != DrawTool::Text {
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
                        self.save_undo_state();
                        let mut shape = FreeShape::default();
                        shape.id = format!("shape-{}", self.next_id);
                        self.next_id += 1;
                        shape.tool = self.tool;
                        shape.x = x;
                        shape.y = y;
                        shape.w = w;
                        shape.h = h;
                        self.doc.shapes.push(shape);
                        self.selected = vec![self.doc.shapes.len() - 1];
                        self.touch();
                    }
                }
            }
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
                    self.paint_shape(painter, &preview, screen_rect, false, ShapeState::Normal, zoom);
                }
            }
        }

        ui.allocate_rect(canvas_rect, Sense::hover());
    }

    /// Определить визуальное состояние фигуры с учётом множеств наведения/нажатия.
    fn shape_state(
        hovered: &HashSet<usize>,
        pressed: &HashSet<usize>,
        idx: usize,
    ) -> ShapeState {
        if pressed.contains(&idx) {
            ShapeState::Pressed
        } else if hovered.contains(&idx) {
            ShapeState::Hover
        } else {
            ShapeState::Normal
        }
    }

    /// Показать контекстное меню у заданной позиции на экране.
    fn show_context_menu(&mut self, ctx: &egui::Context, menu_pos: Pos2, target_idx: Option<usize>) {
        let has_group = target_idx
            .and_then(|i| self.doc.shapes.get(i))
            .and_then(|s| s.group_id.as_ref())
            .is_some();
        let all_grouped = self.selected.iter()
            .all(|&i| self.doc.shapes.get(i).and_then(|s| s.group_id.as_ref()).is_some());
        let any_grouped = self.selected.iter()
            .any(|&i| self.doc.shapes.get(i).and_then(|s| s.group_id.as_ref()).is_some());
        let has_selection = !self.selected.is_empty();

        let mut close_menu = false;

        egui::Window::new("Контекстное меню")
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .default_pos(menu_pos)
            .show(ctx, |ui| {
                ui.set_min_width(160.0);

                if has_selection && ui.button("Удалить фигуру").clicked() {
                    close_menu = true;
                }

                if has_selection {
                    ui.separator();

                    if ui.button("На передний план").clicked() {
                        self.send_to_front();
                        close_menu = true;
                    }
                    if ui.button("На задний план").clicked() {
                        self.send_to_back();
                        close_menu = true;
                    }
                }

                ui.separator();

                if ui.button("Копировать").clicked() {
                    self.copy_selected();
                    close_menu = true;
                }
                if self.clipboard.is_some() && ui.button("Вставить").clicked() {
                    self.paste_from_clipboard();
                    close_menu = true;
                }

                ui.separator();

                if target_idx.is_some() {
                    if has_group {
                        if ui.button("Разгруппировать").clicked() {
                            self.ungroup_selected();
                            close_menu = true;
                        }
                    } else if self.selected.len() > 1 {
                        if ui.button("Сгруппировать").clicked() {
                            self.group_selected();
                            close_menu = true;
                        }
                    } else if !all_grouped && !any_grouped {
                        if ui.button("Сгруппировать").clicked() {
                            self.group_selected();
                            close_menu = true;
                        }
                    }
                }

                if close_menu {
                    self.context_menu = None;
                }
            });
    }

    /// Сгруппировать выделенные фигуры (присвоить общий group_id).
    fn group_selected(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        self.save_undo_state();
        let group_id = format!("group-{}", self.next_id);
        self.next_id += 1;
        for &idx in &self.selected {
            if let Some(shape) = self.doc.shapes.get_mut(idx) {
                shape.group_id = Some(group_id.clone());
            }
        }
        self.touch();
    }

    /// Разгруппировать выделенные фигуры (убрать group_id).
    fn ungroup_selected(&mut self) {
        self.save_undo_state();
        for &idx in &self.selected {
            if let Some(shape) = self.doc.shapes.get_mut(idx) {
                shape.group_id = None;
            }
        }
        self.touch();
    }

    /// Переместить выделенные фигуры на передний план (в конец списка shapes).
    pub(crate) fn send_to_front(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        self.save_undo_state();
        let selected_ids: Vec<String> = self.selected.iter()
            .filter_map(|&i| self.doc.shapes.get(i).map(|s| s.id.clone()))
            .collect();
        let mut selected_shapes: Vec<FreeShape> = Vec::new();
        let mut remaining: Vec<FreeShape> = Vec::new();
        for (i, shape) in self.doc.shapes.drain(..).enumerate() {
            if self.selected.contains(&i) {
                selected_shapes.push(shape);
            } else {
                remaining.push(shape);
            }
        }
        remaining.extend(selected_shapes);
        self.doc.shapes = remaining;
        self.selected = self.doc.shapes.iter()
            .enumerate()
            .filter_map(|(i, s)| if selected_ids.contains(&s.id) { Some(i) } else { None })
            .collect();
        self.touch();
    }

    /// Переместить выделенные фигуры на задний план (в начало списка shapes).
    pub(crate) fn send_to_back(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        self.save_undo_state();
        let selected_ids: Vec<String> = self.selected.iter()
            .filter_map(|&i| self.doc.shapes.get(i).map(|s| s.id.clone()))
            .collect();
        let mut selected_shapes: Vec<FreeShape> = Vec::new();
        let mut remaining: Vec<FreeShape> = Vec::new();
        for (i, shape) in self.doc.shapes.drain(..).enumerate() {
            if self.selected.contains(&i) {
                selected_shapes.push(shape);
            } else {
                remaining.push(shape);
            }
        }
        let mut new_shapes = selected_shapes;
        new_shapes.extend(remaining);
        self.doc.shapes = new_shapes;
        self.selected = self.doc.shapes.iter()
            .enumerate()
            .filter_map(|(i, s)| if selected_ids.contains(&s.id) { Some(i) } else { None })
            .collect();
        self.touch();
    }

    /// Удалить все выделенные фигуры.
    pub(crate) fn delete_selected(&mut self) {
        self.save_undo_state();
        let mut keep: Vec<FreeShape> = Vec::new();
        for (i, shape) in self.doc.shapes.drain(..).enumerate() {
            if !self.selected.contains(&i) {
                keep.push(shape);
            }
        }
        self.doc.shapes = keep;
        self.selected.clear();
        self.context_menu = None;
        self.hovered.clear();
        self.pressed.clear();
        self.direct_hover = None;
        self.direct_press = None;
        self.recompute_next_id();
        self.touch();
    }

    /// Прямой topmost hit без распространения (верхняя фигура под курсором).
    fn direct_hit(&self, world: Pos2) -> Option<usize> {
        self.hit_shape(world)
    }

    /// Активное множество: цель + её дети (по иерархии containment) + группа (горизонтально).
    fn active_set(&self, hit: Option<usize>) -> HashSet<usize> {
        let mut set = HashSet::new();
        let Some(idx) = hit else { return set };

        set.insert(idx);

        // Группировка по group_id (горизонтальная связь, существовавшая раньше).
        if let Some(gid) = &self.doc.shapes[idx].group_id {
            for (j, s) in self.doc.shapes.iter().enumerate() {
                if s.group_id.as_ref() == Some(gid) {
                    set.insert(j);
                }
            }
        }

        // Распространение вниз по дереву containment (родитель -> дети).
        if self.is_container(idx) && self.child_react_mode(idx) != ChildReactMode::None {
            self.collect_descendants(idx, &mut set);
        }

        set
    }

    /// Все подходящие потомки (рекурсивно вниз).
    fn collect_descendants(&self, idx: usize, out: &mut HashSet<usize>) {
        let parent_id = &self.doc.shapes[idx].id;
        let mode = self.child_react_mode(idx);
        let allow = &self.doc.shapes[idx].child_react_allow;

        for (j, child) in self.doc.shapes.iter().enumerate() {
            if child.parent_id.as_ref() != Some(parent_id) {
                continue;
            }

            let allowed = match mode {
                ChildReactMode::All => child.reacts_to_parent,
                ChildReactMode::List => child.reacts_to_parent && allow.contains(&child.id),
                ChildReactMode::None => false,
            };

            if allowed {
                out.insert(j);
                self.collect_descendants(j, out);
            }
        }
    }

    /// Является ли фигура рамкой-контейнером.
    fn is_container(&self, idx: usize) -> bool {
        self.doc.shapes.get(idx).map(|s| s.is_container).unwrap_or(false)
    }

    /// Режим распространения состояния на детей.
    fn child_react_mode(&self, idx: usize) -> ChildReactMode {
        self.doc.shapes
            .get(idx)
            .map(|s| s.child_react_mode)
            .unwrap_or(ChildReactMode::All)
    }

    /// Проверить, не создаст ли установка родителя цикл в иерархии.
    fn would_create_cycle(&self, idx: usize, new_parent_id: &str) -> bool {
        let mut cur = self
            .doc
            .shapes
            .iter()
            .position(|s| s.id == new_parent_id);
        while let Some(p) = cur {
            if p == idx {
                return true;
            }
            cur = self.doc.shapes[p]
                .parent_id
                .as_ref()
                .and_then(|id| self.doc.shapes.iter().position(|s| s.id == *id));
        }
        false
    }

    /// Перестроить порядок фигур так, чтобы родитель шел раньше всех потомков.
    /// Необходимо для корректного hit-теста (обратный обход должен находить ребёнка раньше родителя).
    pub(crate) fn reindex_containment(&mut self) {
        let n = self.doc.shapes.len();
        let id_to_idx: std::collections::HashMap<String, usize> = self
            .doc
            .shapes
            .iter()
            .enumerate()
            .map(|(i, s)| (s.id.clone(), i))
            .collect();

        let mut parent_idx: Vec<Option<usize>> = vec![None; n];
        for (i, s) in self.doc.shapes.iter().enumerate() {
            if let Some(pid) = &s.parent_id {
                parent_idx[i] = id_to_idx.get(pid).copied();
            }
        }

        // Topological sort: родитель před ребёнком (Kahn's algorithm).
        let mut remaining_deps: Vec<usize> = vec![0; n];
        for i in 0..n {
            if parent_idx[i].is_some() {
                remaining_deps[i] = 1;
            }
        }

        let mut queue: Vec<usize> = (0..n).filter(|&i| remaining_deps[i] == 0).collect();
        let mut order = Vec::with_capacity(n);

        while let Some(i) = queue.pop() {
            order.push(i);
            for j in 0..n {
                if parent_idx[j] == Some(i) {
                    remaining_deps[j] -= 1;
                    if remaining_deps[j] == 0 {
                        queue.push(j);
                    }
                }
            }
        }

        // Остальные (сироты с пропавшим родителем или циклы) — в конец.
        for i in 0..n {
            if !order.contains(&i) {
                order.push(i);
            }
        }

        // Запомнить старые id по старым индексам до замены вектора.
        let old_ids: Vec<String> = self.doc.shapes.iter().map(|s| s.id.clone()).collect();
        let new_shapes: Vec<FreeShape> = order.into_iter().map(|i| self.doc.shapes[i].clone()).collect();
        let id_to_new_idx: std::collections::HashMap<&str, usize> = new_shapes
            .iter()
            .enumerate()
            .map(|(new_i, s)| (s.id.as_str(), new_i))
            .collect();
        // Перенаправить выделенные индексы на новые позиции по id фигур.
        self.selected.retain_mut(|idx| {
            let old_id = old_ids.get(*idx).map(|s| s.as_str()).unwrap_or("");
            id_to_new_idx.get(old_id).copied().map_or(false, |new_i| { *idx = new_i; true })
        });
        self.doc.shapes = new_shapes;
    }
}

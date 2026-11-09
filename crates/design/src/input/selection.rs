//! Обработка выделения, кликов, перетаскивания фигур и ресайза.

use crate::free_canvas::FreeCanvas;
use crate::design_format::{DrawTool, FreeShape};
use crate::input::ResizeMode;
use egui::{Color32, PointerButton, Pos2, Rect, Stroke, Vec2};

impl FreeCanvas {
    /// Режим прототипа: отслеживание наведения и нажатия (прямой хит + активное множество).
    pub(crate) fn handle_prototype(&mut self, ui: &egui::Ui, canvas_rect: Rect) {
        if self.prototype_mode {
            if let Some(cursor) = ui.ctx().pointer_hover_pos() {
                if canvas_rect.contains(cursor) {
                    let world_cursor = self.screen_to_world(cursor);
                    let direct = self.hit_shape(world_cursor);
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
    }

    /// Единый фрейм выделения вокруг всех выделенных фигур + ручки ресайза + курсоры.
    pub(crate) fn draw_selection(&mut self, ui: &mut egui::Ui, response: &egui::Response, zoom: f32) {
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
                let screen_rect = self.world_rect_to_screen(world_rect);
                let sr = screen_rect;
                let painter = ui.painter();
                painter.rect_stroke(
                    sr.expand(4.0),
                    0.0,
                    Stroke::new(1.5_f32 * zoom, Color32::from_rgb(100, 180, 255)),
                );

                if !self.prototype_mode {
                    let handle_size = 14.0;
                    let hit_size = 28.0;
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

        // Курсор инструмента при наведении на холст (вне ресайза).
        if response.hovered() && self.resize_mode.is_none() {
            match self.tool {
                DrawTool::Select => ui.ctx().set_cursor_icon(egui::CursorIcon::Default),
                _ => ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair),
            }
        }
    }

    /// Клик по холсту: текстовый инструмент — разместить текстовую фигуру.
    pub(crate) fn handle_text_click(&mut self, ui: &egui::Ui, response: &egui::Response) {
        if response.clicked() && self.tool == DrawTool::Text {
            if let Some(pos) = response.interact_pointer_pos() {
                self.save_undo_state();
                let mut shape = FreeShape::default();
                shape.id = format!("shape-{}", self.next_id);
                self.next_id += 1;
                shape.tool = DrawTool::Text;
                let wp = self.screen_to_world(pos);
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
            }
        }
    }

    /// Клик по холсту: выбор инструмента — выделить фигуру (или несколько с Ctrl).
    pub(crate) fn handle_select_click(&mut self, ui: &egui::Ui, response: &egui::Response) {
        if response.clicked() && self.tool == DrawTool::Select {
            if let Some(pos) = response.interact_pointer_pos() {
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
                        let sr = self.world_rect_to_screen(world_rect);
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
                    let world_pos = self.screen_to_world(pos);
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
                    self.drag_start = response.interact_pointer_pos();
                }
            }
        }
    }

    /// Начало перетаскивания / рисования (только левая кнопка).
    pub(crate) fn handle_drag_start(&mut self, ui: &egui::Ui, response: &egui::Response) {
        let middle_down = ui.ctx().input(|i| i.pointer.button_down(PointerButton::Middle));
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
                        let sr = self.world_rect_to_screen(world_rect);
                        let hit_size = 28.0;
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
                                for &idx in &self.selected {
                                    if let Some(shape) = self.doc.shapes.get(idx) {
                                        self.resize_origin.push((idx, shape.x, shape.y, shape.w, shape.h));
                                    }
                                }
                                self.save_undo_state();
                                break;
                            }
                        }
                    }
                }
            }

            if self.resize_mode.is_none() && self.tool == DrawTool::Select {
                if let Some(pos) = self.drag_start {
                    let world_pos = self.screen_to_world(pos);
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
            } else if self.resize_mode.is_none() {
                self.drag_shape_pos = None;
            }
        }
    }

    /// Перетаскивание выделенных фигур или отрисовка selection box.
    pub(crate) fn handle_drag(&mut self, ui: &mut egui::Ui, response: &egui::Response, magnetic_lines: &mut Vec<(f32, bool)>) {
        let middle_down = ui.ctx().input(|i| i.pointer.button_down(PointerButton::Middle));
        if response.dragged() && !middle_down && self.tool == DrawTool::Select && self.resize_mode.is_none() {
            if !self.drag_history_saved {
                self.save_undo_state();
                self.drag_history_saved = true;
            }
            if let Some(pos) = response.interact_pointer_pos() {
                if let Some((idx, origin)) = self.drag_shape_pos {
                    let world_cursor = self.screen_to_world(pos);
                    let drag_world_start = self.screen_to_world(self.drag_start.unwrap_or(pos));
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
                    let world_start = self.screen_to_world(self.drag_start.unwrap_or(pos));
                    let world_cursor = self.screen_to_world(pos);
                    let x = world_start.x.min(world_cursor.x);
                    let y = world_start.y.min(world_cursor.y);
                    let w = (world_cursor.x - world_start.x).abs();
                    let h = (world_cursor.y - world_start.y).abs();
                    if w > 1.0 && h > 1.0 {
                        let world_rect = Rect::from_min_size(
                            Pos2::new(x, y),
                            Vec2::new(w.max(1.0), h.max(1.0)),
                        );
                        let screen_rect = self.world_rect_to_screen(world_rect);
                        let painter = ui.painter();
                        painter.rect_stroke(
                            screen_rect,
                            0.0,
                            Stroke::new(2.0_f32, Color32::from_rgb(0, 217, 255)),
                        );
                    }
                }
            }
        }
    }

    /// Изменение размера выделенных фигур за угол или сторону.
    pub(crate) fn handle_resize(&mut self, ui: &mut egui::Ui, response: &egui::Response, magnetic_lines: &mut Vec<(f32, bool)>) {
        let middle_down = ui.ctx().input(|i| i.pointer.button_down(PointerButton::Middle));
        if response.dragged() && !middle_down && self.resize_mode.is_some() {
            let mode = self.resize_mode.unwrap();
            if let Some(pos) = response.interact_pointer_pos() {
                let world_cursor = self.screen_to_world(pos);
                let drag_world_start = self.screen_to_world(self.drag_start.unwrap_or(pos));
                let dx = world_cursor.x - drag_world_start.x;
                let dy = world_cursor.y - drag_world_start.y;

                for &idx in &self.selected {
                    let (_orig_idx, sx, sy, sw, sh) = self
                        .resize_origin
                        .iter()
                        .find(|(i, _, _, _, _)| *i == idx)
                        .copied()
                        .unwrap_or((idx, 0.0, 0.0, 1.0, 1.0));

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

                    if self.magnetic {
                        let w = nw.max(1.0);
                        let h = nh.max(1.0);
                        let thresh = 6.0;
                        let unlock_thresh = thresh * 2.0;

                        let move_top = matches!(mode, ResizeMode::N | ResizeMode::NW | ResizeMode::NE);
                        let move_bottom = matches!(mode, ResizeMode::S | ResizeMode::SW | ResizeMode::SE);
                        let move_left = matches!(mode, ResizeMode::W | ResizeMode::NW | ResizeMode::SW);
                        let move_right = matches!(mode, ResizeMode::E | ResizeMode::NE | ResizeMode::SE);

                        let raw_top = ny;
                        let raw_bottom = ny + h;
                        let raw_left = nx;
                        let raw_right = nx + w;

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
    }

    /// Завершение перетаскивания — создать новую фигуру или выделить по box.
    pub(crate) fn handle_drag_stopped(&mut self, ui: &mut egui::Ui, response: &egui::Response, magnetic_lines: &mut Vec<(f32, bool)>) {
        let middle_down = ui.ctx().input(|i| i.pointer.button_down(PointerButton::Middle));
        if response.drag_stopped() && !middle_down {
            let was_resizing = self.resize_mode.is_some();
            self.resize_mode = None;
            self.resize_origin.clear();
            self.resize_mag_locked_x = None;
            self.resize_mag_locked_y = None;

            if !was_resizing && self.tool == DrawTool::Select && self.drag_shape_pos.is_none() {
                // Selection box: выделить все фигуры внутри резинового прямоугольника
                if let (Some(start), Some(end)) = (self.drag_start, response.interact_pointer_pos()) {
                    let world_start = self.screen_to_world(start);
                    let world_end = self.screen_to_world(end);
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
                    let world_start = self.screen_to_world(start);
                    let world_end = self.screen_to_world(end);
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
    }
}

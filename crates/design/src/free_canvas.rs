//! Figma-подобный свободный холст: рисуем прямоугольники, линии, эллипсы, текст,
//! перетаскиваем элементы, магнитное выравнивание, панорамирование, зум колёсиком,
//! экспорт JSON для Агента.

use crate::design_format::*;
use egui::{
    Color32, FontId, Key, PointerButton, Pos2, Rect, Rounding, Sense, Shape, Stroke, Vec2,
};

/// Состояние свободного холста.
pub struct FreeCanvas {
    pub doc: FreeCanvasDoc,
    pub selected: Option<usize>,
    pub tool: DrawTool,
    pub magnetic: bool,
    pub camera: Vec2,
    pub zoom: f32,
    pub pan_start: Option<Pos2>,
    pub pan_camera_start: Vec2,
    pub drag_start: Option<Pos2>,
    pub drag_shape_pos: Option<(usize, Pos2)>,
    pub next_id: u64,
    pub grid_size: f32,
}

impl Default for FreeCanvas {
    fn default() -> Self {
        Self {
            doc: FreeCanvasDoc::default(),
            selected: None,
            tool: DrawTool::Select,
            magnetic: false,
            camera: Vec2::ZERO,
            zoom: 1.0,
            pan_start: None,
            pan_camera_start: Vec2::ZERO,
            drag_start: None,
            drag_shape_pos: None,
            next_id: 1,
            grid_size: 20.0,
        }
    }
}

impl FreeCanvas {
    pub fn new() -> Self {
        Self::default()
    }

    /// Отобразить панель свойств слева и холст справа.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        // Удаление выделенного по клавише Delete
        if ui.ctx().input(|i| i.key_pressed(Key::Delete)) {
            self.delete_selected();
        }

        ui.columns(2, |cols: &mut [egui::Ui]| {
            cols[0].vertical(|ui| {
                ui.set_min_width(300.0);
                self.properties_panel(ui);
            });
            cols[1].vertical(|ui| {
                self.canvas_panel(ui);
            });
        });
    }

    /// Панель инструментов и свойств выделенной фигуры.
    fn properties_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Инструменты");
        ui.separator();

        for (tool, label) in [
            (DrawTool::Select, "Выделить"),
            (DrawTool::Rect, "Прямоугольник"),
            (DrawTool::Line, "Линия"),
            (DrawTool::Ellipse, "Эллипс"),
            (DrawTool::Text, "Текст"),
        ] {
            let active = self.tool == tool;
            if ui.selectable_label(active, label).clicked() {
                self.tool = tool;
            }
        }

        ui.separator();
        ui.checkbox(&mut self.magnetic, "Магнитное выравнивание");
        ui.separator();
        ui.label("Цвет фона холста:");
        Self::color_picker(ui, &mut self.doc.bg);
        ui.separator();
        ui.label(format!("Фигуры: {}", self.doc.shapes.len()));
        if ui.button("Удалить выделенное").clicked() {
            self.delete_selected();
        }
        if ui.button("Очистить").clicked() {
            self.doc.shapes.clear();
            self.selected = None;
        }

        ui.separator();
        ui.heading("Свойства фигуры");
        ui.separator();

        match self.selected {
            Some(idx) => {
                let shape = &mut self.doc.shapes[idx];
                ui.label(format!("Фигура #{}", shape.id));
                ui.separator();

                ui.label("Инструмент:");
                egui::ComboBox::from_label("")
                    .selected_text(format!("{:?}", shape.tool))
                    .show_ui(ui, |ui| {
                        for t in [DrawTool::Rect, DrawTool::Line, DrawTool::Ellipse, DrawTool::Text] {
                            ui.selectable_value(&mut shape.tool, t, format!("{:?}", t));
                        }
                    });

                ui.horizontal(|ui| {
                    ui.label("X:");
                    ui.add(egui::DragValue::new(&mut shape.x).speed(1.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Y:");
                    ui.add(egui::DragValue::new(&mut shape.y).speed(1.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Ширина:");
                    ui.add(
                        egui::DragValue::new(&mut shape.w)
                            .speed(1.0)
                            .range(0.0..=10000.0),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Высота:");
                    ui.add(
                        egui::DragValue::new(&mut shape.h)
                            .speed(1.0)
                            .range(0.0..=10000.0),
                    );
                });

                ui.separator();
                ui.label("Заливка:");
                if let Some(fill) = &mut shape.fill {
                    Self::color_picker(ui, fill);
                } else {
                    if ui.button("Добавить заливку").clicked() {
                        shape.fill = Some(Rgba { r: 0, g: 0, b: 0, a: 255 });
                    }
                }
                ui.horizontal(|ui| {
                    ui.label("Обводка:");
                    Self::color_picker(ui, &mut shape.stroke);
                });
                ui.add(
                    egui::DragValue::new(&mut shape.stroke_width)
                        .prefix("Толщина обводки: ")
                        .speed(0.1)
                        .range(0.0..=20.0),
                );

                ui.separator();
                ui.label("Скругление углов:");
                // Мастер-контроль «все углы»
                let mut all = shape.corner_radius;
                if ui
                    .add(
                        egui::DragValue::new(&mut all)
                            .prefix("Все углы: ")
                            .speed(0.5)
                            .range(0.0..=100.0),
                    )
                    .changed()
                {
                    shape.corner_tl = all;
                    shape.corner_tr = all;
                    shape.corner_bl = all;
                    shape.corner_br = all;
                    shape.corner_radius = all;
                }
                ui.horizontal(|ui| {
                    ui.label("Левый верхний:");
                    ui.add(
                        egui::DragValue::new(&mut shape.corner_tl)
                            .speed(0.5)
                            .range(0.0..=100.0),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Правый верхний:");
                    ui.add(
                        egui::DragValue::new(&mut shape.corner_tr)
                            .speed(0.5)
                            .range(0.0..=100.0),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Левый нижний:");
                    ui.add(
                        egui::DragValue::new(&mut shape.corner_bl)
                            .speed(0.5)
                            .range(0.0..=100.0),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Правый нижний:");
                    ui.add(
                        egui::DragValue::new(&mut shape.corner_br)
                            .speed(0.5)
                            .range(0.0..=100.0),
                    );
                });

                if shape.tool == DrawTool::Text {
                    ui.separator();
                    ui.label("Текст:");
                    ui.text_edit_singleline(&mut shape.text);
                    ui.add(
                        egui::DragValue::new(&mut shape.font_size)
                            .prefix("Размер шрифта: ")
                            .speed(0.5)
                            .range(4.0..=200.0),
                    );
                    ui.horizontal(|ui| {
                        ui.label("Цвет текста:");
                        Self::color_picker(ui, &mut shape.text_color);
                    });
                    ui.label("Выравнивание:");
                    egui::ComboBox::from_label("")
                        .selected_text(match shape.text_align {
                            TextAlign::Left => "По левому",
                            TextAlign::Center => "По центру",
                            TextAlign::Right => "По правому",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut shape.text_align, TextAlign::Left, "По левому");
                            ui.selectable_value(&mut shape.text_align, TextAlign::Center, "По центру");
                            ui.selectable_value(&mut shape.text_align, TextAlign::Right, "По правому");
                        });
                }

                ui.separator();
                if ui.button("Удалить фигуру").clicked() {
                    self.doc.shapes.remove(idx);
                    self.selected = None;
                }
            }
            None => {
                ui.label("Ничего не выбрано.");
                ui.label("Нажмите на фигуру на холсте, чтобы выбрать.");
            }
        }
    }

    fn canvas_panel(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size();
        let canvas_rect = egui::Rect::from_min_size(ui.cursor().min, available);

        // Обработка ввода сначала (избегаем конфликта заимствований с painter)
        let response = ui.allocate_rect(canvas_rect, Sense::click_and_drag());
        let pointer_pos = response.hover_pos();
        let painter = ui.painter();

        let middle_down = ui.ctx().input(|i| i.pointer.button_down(PointerButton::Middle));

        // Приближение/отдаление колёсиком мыши к курсору
        let scroll = ui.ctx().input(|i| i.raw_scroll_delta);
        if scroll.y.abs() > 0.001 {
            if let Some(cursor) = ui.ctx().pointer_hover_pos() {
                if canvas_rect.contains(cursor) {
                    let zoom = self.zoom.clamp(0.1, 10.0);
                    let camera = self.camera;
                    let world_cursor = (cursor - camera) / zoom;
                    let factor = (-scroll.y * 0.01).exp();
                    let new_zoom = (zoom * factor).clamp(0.1, 10.0);
                    self.camera = cursor - world_cursor * new_zoom;
                    self.zoom = new_zoom;
                    ui.ctx().request_repaint();
                }
            }
        }

        let camera = self.camera;
        let zoom = self.zoom.clamp(0.1, 10.0);

        // Панорамирование средней кнопкой мыши
        if response.drag_started() && middle_down {
            self.pan_start = pointer_pos;
            self.pan_camera_start = self.camera;
        }
        if response.dragged() && middle_down {
            if let (Some(start), Some(pos)) = (self.pan_start, pointer_pos) {
                self.camera = self.pan_camera_start + (pos - start);
            }
        }
        if response.drag_stopped() && middle_down {
            self.pan_start = None;
        }

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
                    screen_rect.expand(3.0 / zoom),
                    2.0 / zoom,
                    Stroke::new(1.5_f32 / zoom.max(0.1), Color32::from_rgb(100, 180, 255)),
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
            if let Some(pos) = pointer_pos {
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
                } else if self.tool == DrawTool::Select {
                    if let Some(pos) = pointer_pos {
                        let world_pos = screen_to_world(pos);
                        self.selected = self.hit_shape(world_pos);
                        if let Some(idx) = self.selected {
                            if let Some(shape) = self.doc.shapes.get(idx) {
                                self.drag_shape_pos = Some((idx, Pos2::new(shape.x, shape.y)));
                            }
                        } else {
                            self.drag_shape_pos = None;
                        }
                        self.drag_start = pointer_pos;
                    }
                }
            }
        }

        // Начало перетаскивания / рисования (только левая кнопка)
        if response.drag_started() && !middle_down {
            self.drag_start = pointer_pos;
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
            if let Some(pos) = pointer_pos {
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
                    }
                }
            }
        }

        // Завершение перетаскивания — создать новую фигуру (кроме Select и Text)
        if response.drag_stopped() && !middle_down {
            if self.tool != DrawTool::Select && self.tool != DrawTool::Text {
                if let (Some(start), Some(end)) = (self.drag_start, pointer_pos) {
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

    /// Проверка попадания курсора в фигуру (мировые координаты).
    fn hit_shape(&self, pos: Pos2) -> Option<usize> {
        for (i, shape) in self.doc.shapes.iter().enumerate().rev() {
            let rect = Rect::from_min_size(
                Pos2::new(shape.x, shape.y),
                Vec2::new(shape.w.max(1.0), shape.h.max(1.0)),
            );
            match shape.tool {
                DrawTool::Line => {
                    let p1 = Pos2::new(shape.x, shape.y);
                    let p2 = Pos2::new(shape.x + shape.w, shape.y + shape.h);
                    if dist_point_segment(pos, p1, p2) < 6.0 {
                        return Some(i);
                    }
                }
                _ => {
                    if rect.contains(pos) {
                        return Some(i);
                    }
                }
            }
        }
        None
    }

    fn delete_selected(&mut self) {
        if let Some(idx) = self.selected.take() {
            if idx < self.doc.shapes.len() {
                self.doc.shapes.remove(idx);
            }
        }
    }

    /// Нарисовать одну фигуру.
    fn paint_shape(
        &self,
        painter: &egui::Painter,
        shape: &FreeShape,
        rect: Rect,
        _is_selected: bool,
    ) {
        let fill = shape
            .fill
            .map(|c| Color32::from_rgb(c.r, c.g, c.b))
            .unwrap_or(Color32::TRANSPARENT);
        let stroke_color = Color32::from_rgb(shape.stroke.r, shape.stroke.g, shape.stroke.b);
        let stroke = Stroke::new(shape.stroke_width.max(0.0), stroke_color);

        let rounding = if shape.corner_tl == shape.corner_tr
            && shape.corner_tr == shape.corner_bl
            && shape.corner_bl == shape.corner_br
        {
            Rounding::same(shape.corner_tl)
        } else {
            Rounding {
                nw: shape.corner_tl,
                ne: shape.corner_tr,
                sw: shape.corner_bl,
                se: shape.corner_br,
            }
        };

        match shape.tool {
            DrawTool::Select => {}
            DrawTool::Rect | DrawTool::Text => {
                painter.rect_filled(rect, rounding, fill);
                if shape.stroke_width > 0.0 {
                    painter.rect_stroke(rect, rounding, stroke);
                }
            }
            DrawTool::Ellipse => {
                let center = rect.center();
                let radii = Vec2::new(rect.width() / 2.0, rect.height() / 2.0);
                painter.add(Shape::ellipse_filled(center, radii, fill));
                if shape.stroke_width > 0.0 {
                    painter.add(Shape::ellipse_stroke(center, radii, stroke));
                }
            }
            DrawTool::Line => {
                let p1 = Pos2::new(rect.min.x, rect.min.y);
                let p2 = Pos2::new(rect.max.x, rect.max.y);
                painter.line_segment([p1, p2], stroke);
            }
        }

        // Текст
        if !shape.text.is_empty() {
            let text_color =
                Color32::from_rgb(shape.text_color.r, shape.text_color.g, shape.text_color.b);
            let font = FontId::proportional(shape.font_size.max(6.0));
            let galley = painter.layout_no_wrap(shape.text.clone(), font, text_color);
            let pos = match shape.text_align {
                TextAlign::Left => rect.min + Vec2::new(8.0, rect.height() / 2.0),
                TextAlign::Center => rect.center() - Vec2::new(galley.size().x / 2.0, galley.size().y / 2.0),
                TextAlign::Right => rect.max - Vec2::new(galley.size().x + 8.0, rect.height() / 2.0),
            };
            painter.galley(pos, galley, text_color);
        }
    }

    fn color_picker(ui: &mut egui::Ui, color: &mut Rgba) {
        let mut egui_color = Color32::from_rgba_unmultiplied(color.r, color.g, color.b, color.a);
        ui.color_edit_button_srgba(&mut egui_color);
        color.r = egui_color.r();
        color.g = egui_color.g();
        color.b = egui_color.b();
        color.a = egui_color.a();
    }
}

/// Расстояние от точки до отрезка.
fn dist_point_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let ap = p - a;
    let mut t = (ap.x * ab.x + ap.y * ab.y) / (ab.x * ab.x + ab.y * ab.y).max(1e-6);
    t = t.clamp(0.0, 1.0);
    let closest = a + ab * t;
    (p - closest).length()
}

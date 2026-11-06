//! Figma-подобный свободный холст: рисуем прямоугольники, линии, эллипсы, текст,
//! перетаскиваем элементы, магнитное выравнивание, экспорт JSON для Агента.

use crate::design_format::*;
use egui::{
    Align2, Color32, FontId, Pos2, Rect, Sense, Shape, Stroke, Vec2,
};

/// Состояние свободного холста.
pub struct FreeCanvas {
    pub doc: FreeCanvasDoc,
    pub selected: Option<usize>,
    pub tool: DrawTool,
    pub magnetic: bool,
    pub drag_start: Option<Pos2>,
    pub drag_shape_pos: Option<(usize, Pos2)>,
    pub next_id: u64,
    pub show_grid: bool,
    pub grid_size: f32,
}

impl Default for FreeCanvas {
    fn default() -> Self {
        Self {
            doc: FreeCanvasDoc::default(),
            selected: None,
            tool: DrawTool::Select,
            magnetic: false,
            drag_start: None,
            drag_shape_pos: None,
            next_id: 1,
            show_grid: true,
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
                ui.add(
                    egui::DragValue::new(&mut shape.corner_radius)
                        .prefix("Скругление: ")
                        .speed(0.5)
                        .range(0.0..=100.0),
                );

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
        let painter = ui.painter();

        // Фон холста
        painter.rect_filled(canvas_rect, 0.0, egui::Color32::from_rgb(
            self.doc.bg.r, self.doc.bg.g, self.doc.bg.b,
        ));

        // Сетка
        if self.doc.show_grid {
            let grid_color = egui::Color32::from_rgb(self.doc.grid.r, self.doc.grid.g, self.doc.grid.b);
            let mut x = canvas_rect.min.x - (canvas_rect.min.x % self.grid_size);
            while x < canvas_rect.max.x {
                painter.line_segment(
                    [Pos2::new(x, canvas_rect.min.y), Pos2::new(x, canvas_rect.max.y)],
                    Stroke::new(0.5_f32, grid_color),
                );
                x += self.grid_size;
            }
            let mut y = canvas_rect.min.y - (canvas_rect.min.y % self.grid_size);
            while y < canvas_rect.max.y {
                painter.line_segment(
                    [Pos2::new(canvas_rect.min.x, y), Pos2::new(canvas_rect.max.x, y)],
                    Stroke::new(0.5_f32, grid_color),
                );
                y += self.grid_size;
            }
        }

        // Магнитные линии (отображаются поверх всего в конце)
        let mut magnetic_lines: Vec<(f32, bool)> = Vec::new(); // (координата, вертикальная)

        // Отрисовать все фигуры
        for (i, shape) in self.doc.shapes.iter().enumerate() {
            let is_selected = self.selected == Some(i);
            let rect = Rect::from_min_size(
                Pos2::new(shape.x, shape.y),
                Vec2::new(shape.w.max(1.0), shape.h.max(1.0)),
            );
            let screen_rect = rect.intersect(canvas_rect);
            if screen_rect.is_positive() {
                self.paint_shape(painter, shape, screen_rect, is_selected);
            }
        }

        // Выделение
        if let Some(idx) = self.selected {
            if let Some(shape) = self.doc.shapes.get(idx) {
                let rect = Rect::from_min_size(
                    Pos2::new(shape.x, shape.y),
                    Vec2::new(shape.w.max(1.0), shape.h.max(1.0)),
                );
                painter.rect_stroke(
                    rect.expand(3.0),
                    2.0,
                    Stroke::new(1.5_f32, Color32::from_rgb(100, 180, 255)),
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
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                if self.tool == DrawTool::Text {
                    let mut shape = FreeShape::default();
                    shape.id = format!("shape-{}", self.next_id);
                    self.next_id += 1;
                    shape.tool = DrawTool::Text;
                    shape.x = pos.x;
                    shape.y = pos.y;
                    shape.w = 140.0;
                    shape.h = 30.0;
                    shape.text = "Текст".to_owned();
                    self.doc.shapes.push(shape);
                    self.selected = Some(self.doc.shapes.len() - 1);
                    self.drag_start = None;
                    self.drag_shape_pos = None;
                } else if self.tool == DrawTool::Select {
                    if let Some(pos) = response.interact_pointer_pos() {
                        self.selected = self.hit_shape(pos);
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
        }

        // Начало перетаскивания / рисования
        if response.drag_started() && self.tool != DrawTool::Text {
            self.drag_start = response.interact_pointer_pos();
            if self.tool == DrawTool::Select {
                if let Some(pos) = self.drag_start {
                    self.selected = self.hit_shape(pos);
                    if let Some(idx) = self.selected {
                        if let Some(shape) = self.doc.shapes.get(idx) {
                            self.drag_shape_pos = Some((idx, Pos2::new(shape.x, shape.y)));
                        }
                    } else {
                        self.drag_shape_pos = None;
                    }
                }
            } else {
                self.drag_shape_pos = None;
            }
        }

        // Перетаскивание
        if response.dragged() && self.tool == DrawTool::Select {
            if let Some(pos) = response.interact_pointer_pos() {
                if let Some((idx, origin)) = self.drag_shape_pos {
                    let mut nx = origin.x + (pos.x - self.drag_start.unwrap_or(pos).x);
                    let mut ny = origin.y + (pos.y - self.drag_start.unwrap_or(pos).y);

                    if self.magnetic {
                        if let Some(shape) = self.doc.shapes.get(idx) {
                            let w = shape.w.max(1.0);
                            let h = shape.h.max(1.0);
                            let thresh = 6.0;
                            // Магнитится к краям других фигур
                            for (j, other) in self.doc.shapes.iter().enumerate() {
                                if j == idx {
                                    continue;
                                }
                                let or = Rect::from_min_size(
                                    Pos2::new(other.x, other.y),
                                    Vec2::new(other.w.max(1.0), other.h.max(1.0)),
                                );
                                // Левый край к левому/правому/центру
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
                                // Верхний край к верхнему/нижнему/центру
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
        if response.drag_stopped() {
            if self.tool != DrawTool::Select && self.tool != DrawTool::Text {
                if let (Some(start), Some(end)) = (self.drag_start, response.interact_pointer_pos()) {
                    let mut x = start.x.min(end.x);
                    let mut y = start.y.min(end.y);
                    let mut w = (end.x - start.x).abs();
                    let mut h = (end.y - start.y).abs();

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

        // Магнитные красные линии выравнивания
        if self.magnetic {
            for (coord, vertical) in &magnetic_lines {
                if *vertical {
                    painter.line_segment(
                        [Pos2::new(*coord, canvas_rect.min.y), Pos2::new(*coord, canvas_rect.max.y)],
                        Stroke::new(1.0_f32, Color32::from_rgb(255, 0, 0)),
                    );
                } else {
                    painter.line_segment(
                        [Pos2::new(canvas_rect.min.x, *coord), Pos2::new(canvas_rect.max.x, *coord)],
                        Stroke::new(1.0_f32, Color32::from_rgb(255, 0, 0)),
                    );
                }
            }
        }

        // Предварительный вид фигуры во время рисования
        if self.tool != DrawTool::Select && self.tool != DrawTool::Text {
            if let (Some(start), Some(pos)) = (self.drag_start, response.hover_pos()) {
                let x = start.x.min(pos.x);
                let y = start.y.min(pos.y);
                let w = (pos.x - start.x).abs();
                let h = (pos.y - start.y).abs();
                if w > 2.0 && h > 2.0 {
                    let preview = FreeShape {
                        tool: self.tool,
                        x, y, w, h,
                        ..FreeShape::default()
                    };
                    let rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w.max(1.0), h.max(1.0)));
                    self.paint_shape(painter, &preview, rect, false);
                }
            }
        }

        ui.allocate_rect(canvas_rect, Sense::hover());
    }

    /// Проверка попадания курсора в фигуру.
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

        match shape.tool {
            DrawTool::Select => {}
            DrawTool::Rect | DrawTool::Text => {
                painter.rect_filled(rect, shape.corner_radius, fill);
                if shape.stroke_width > 0.0 {
                    painter.rect_stroke(rect, shape.corner_radius, stroke);
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

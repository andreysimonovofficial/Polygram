//! Figma-подобный свободный холст: рисуем прямоугольники, линии, эллипсы, текст,
//! перетаскиваем элементы, магнитное выравнивание, панорамирование, зум колёсиком,
//! автосохранение последнего проекта и двусторонний редактор кода для Агента.

use crate::design_format::*;
use eframe::egui;
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
    pub sidebar_width: f32,
    pub settings_loaded: bool,
    pub doc_dirty: bool,
    pub show_code_sidebar: bool,
    pub code_text: String,
    pub last_applied_code: String,
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
            sidebar_width: 300.0,
            settings_loaded: false,
            doc_dirty: false,
            show_code_sidebar: false,
            code_text: String::new(),
            last_applied_code: String::new(),
        }
    }
}

impl FreeCanvas {
    pub fn new() -> Self {
        Self::default()
    }

    /// Загрузить настройки и последний проект из хранилища eframe.
    fn load_settings(&mut self, storage: &dyn eframe::Storage) {
        if let Some(s) = storage.get_string("polygram_design/sidebar_width") {
            if let Ok(w) = s.parse::<f32>() {
                self.sidebar_width = w.clamp(200.0, 1000.0);
            }
        }
        if let Some(json) = storage.get_string("polygram_design/last_doc") {
            if let Ok(doc) = FreeCanvasDoc::load_from_json(&json) {
                self.doc = doc;
                self.recompute_next_id();
            }
        }
    }

    /// Сохранить настройки и последний проект в хранилище eframe.
    pub fn save_settings(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string("polygram_design/sidebar_width", self.sidebar_width.to_string());
        storage.set_string("polygram_design/last_doc", self.doc.to_json().unwrap_or_default());
    }

    /// Пересчитать next_id на основе загруженных фигур.
    fn recompute_next_id(&mut self) {
        let mut max: u64 = 0;
        for shape in &self.doc.shapes {
            if let Some(num) = shape.id.strip_prefix("shape-") {
                if let Ok(n) = num.parse::<u64>() {
                    if n > max {
                        max = n;
                    }
                }
            }
        }
        self.next_id = max + 1;
    }

    /// Отметить документ изменённым (для автосохранения).
    fn touch(&mut self) {
        self.doc_dirty = true;
    }

    /// Отобразить нижнюю панель инструментов, правую боковую панель свойств,
    /// левую боковую панель кода и холст.
    pub fn ui(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Удаление выделенного по клавише Delete
        if ctx.input(|i| i.key_pressed(Key::Delete)) {
            self.delete_selected();
            self.touch();
        }

        // Загрузка настроек при первом кадре
        if !self.settings_loaded {
            if let Some(storage) = frame.storage() {
                self.load_settings(storage);
            }
            self.settings_loaded = true;
        }

        // Нижняя панель инструментов
        egui::TopBottomPanel::bottom("tools")
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.group(|ui| {
                        ui.label("Инструмент:");
                        for (tool, label) in [
                            (DrawTool::Select, "Выделить"),
                            (DrawTool::Rect, "Прямоугольник"),
                            (DrawTool::Line, "Линия"),
                            (DrawTool::Ellipse, "Элипс"),
                            (DrawTool::Text, "Текст"),
                        ] {
                            let active = self.tool == tool;
                            if ui.selectable_label(active, label).clicked() {
                                self.tool = tool;
                            }
                        }
                    });
                    ui.separator();
                    if ui.button("Код для агента").clicked() {
                        self.show_code_sidebar = true;
                        self.code_text = self.doc.to_json().unwrap_or_default();
                        self.last_applied_code = self.code_text.clone();
                    }
                });
            });
        // Левая боковая панель: редактор кода JSON для Агента (двусторонний, на лету)
        if self.show_code_sidebar {
            egui::SidePanel::left("code_editor")
                .default_width(520.0)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.heading("Код для агента (JSON)");
                    ui.separator();

                    // Если дизайн изменился через UI, синхронизируем код в поле (дизайн → код)
                    let current_json = self.doc.to_json().unwrap_or_default();
                    if self.code_text != current_json {
                        self.code_text = current_json.clone();
                        self.last_applied_code = current_json;
                    }

                    // Поле кода растягивается на всю ширину сайдбара с вертикальным скроллбаром.
                    let available_width = ui.available_width();
                    egui::TextEdit::multiline(&mut self.code_text)
                        .desired_rows(18)
                        .desired_width(available_width)
                        .show(ui);

                    // Применяем изменения из кода в дизайн «на лету» (код → дизайн),
                    // но только когда текст реально изменился, чтобы не сбрасывать курсор.
                    if self.code_text != self.last_applied_code {
                        if let Ok(new_doc) = FreeCanvasDoc::load_from_json(&self.code_text) {
                            self.doc = new_doc;
                            self.recompute_next_id();
                            self.selected = None;
                            self.touch();
                            self.last_applied_code = self.code_text.clone();
                        }
                    }

                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Обновить из дизайна").clicked() {
                            self.code_text = self.doc.to_json().unwrap_or_default();
                            self.last_applied_code = self.code_text.clone();
                        }
                        if ui.button("Применить и закрыть").clicked() {
                            if let Ok(new_doc) = FreeCanvasDoc::load_from_json(&self.code_text) {
                                self.doc = new_doc;
                                self.recompute_next_id();
                                self.selected = None;
                                self.touch();
                            }
                            self.show_code_sidebar = false;
                        }
                        if ui.button("Закрыть").clicked() {
                            self.show_code_sidebar = false;
                        }
                    });
                });
        }

        // Правая боковая панель: свойства
        let response = egui::SidePanel::right("properties")
            .resizable(true)
            .default_width(self.sidebar_width)
            .min_width(200.0)
            .max_width(1000.0)
            .show(ctx, |ui| {
                self.properties_panel(ui);
            });
        let panel_width = response.response.rect.width();
        if (panel_width - self.sidebar_width).abs() > 0.5 {
            self.sidebar_width = panel_width;
        }

        // Центральная область: холст
        egui::CentralPanel::default().show(ctx, |ui| {
            self.canvas_panel(ui);
        });

        // Автосохранение последнего проекта при изменениях
        if self.doc_dirty {
            if let Some(storage) = frame.storage_mut() {
                self.save_settings(storage);
            }
            self.doc_dirty = false;
        }
    }

    /// Панель свойств выделенной фигуры (справа).
    fn properties_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Свойства");
        ui.separator();

        ui.label("Цвет фона холста:");
        Self::color_picker(ui, &mut self.doc.bg);
        self.touch();
        ui.separator();

        ui.checkbox(&mut self.magnetic, "Магнитное выравнивание");
        ui.separator();

        ui.label(format!("Фигуры: {}", self.doc.shapes.len()));
        if ui.button("Удалить выделенное").clicked() {
            self.delete_selected();
            self.touch();
        }
        if ui.button("Очистить").clicked() {
            self.doc.shapes.clear();
            self.selected = None;
            self.touch();
        }

        ui.separator();
        ui.heading("Свойства фигуры");
        ui.separator();

        match self.selected {
            Some(idx) => {
                let (changed, delete) = Self::edit_shape(ui, &mut self.doc.shapes[idx]);
                if delete {
                    self.doc.shapes.remove(idx);
                    self.selected = None;
                    self.touch();
                } else if changed {
                    self.touch();
                }
            }
            None => {
                ui.label("Ничего не выбрано.");
                ui.label("Нажмите на фигуру на холсте, чтобы выбрать.");
            }
        }
    }

    /// Редактировать свойства одной фигуры. Возвращает (изменено, удалить).
    fn edit_shape(ui: &mut egui::Ui, shape: &mut FreeShape) -> (bool, bool) {
        let mut changed = false;
        let mut delete = false;

        ui.label(format!("Фигура #{}", shape.id));
        ui.separator();

        ui.label("Инструмент:");
        let mut tool = shape.tool;
        egui::ComboBox::from_label("")
            .selected_text(format!("{:?}", tool))
            .show_ui(ui, |ui| {
                for t in [DrawTool::Rect, DrawTool::Line, DrawTool::Ellipse, DrawTool::Text] {
                    if ui.selectable_value(&mut tool, t, format!("{:?}", t)).changed() {
                        changed = true;
                    }
                }
            });
        shape.tool = tool;

        ui.horizontal(|ui| {
            ui.label("X:");
            changed |= ui.add(egui::DragValue::new(&mut shape.x).speed(1.0)).changed();
        });
        ui.horizontal(|ui| {
            ui.label("Y:");
            changed |= ui.add(egui::DragValue::new(&mut shape.y).speed(1.0)).changed();
        });
        ui.horizontal(|ui| {
            ui.label("Ширина:");
            changed |= ui
                .add(
                    egui::DragValue::new(&mut shape.w)
                        .speed(1.0)
                        .range(0.0..=10000.0),
                )
                .changed();
        });
        ui.horizontal(|ui| {
            ui.label("Высота:");
            changed |= ui
                .add(
                    egui::DragValue::new(&mut shape.h)
                        .speed(1.0)
                        .range(0.0..=10000.0),
                )
                .changed();
        });

        ui.separator();
        ui.label("Заливка:");
        if let Some(fill) = &mut shape.fill {
            let mut color = Color32::from_rgb(fill.r, fill.g, fill.b);
            if ui.color_edit_button_srgba(&mut color).changed() {
                fill.r = color.r();
                fill.g = color.g();
                fill.b = color.b();
                changed = true;
            }
        } else {
            if ui.button("Добавить заливку").clicked() {
                shape.fill = Some(Rgba { r: 0, g: 0, b: 0, a: 255 });
                changed = true;
            }
        }
        ui.horizontal(|ui| {
            ui.label("Обводка:");
            let mut stroke_color = Color32::from_rgb(shape.stroke.r, shape.stroke.g, shape.stroke.b);
            if ui.color_edit_button_srgba(&mut stroke_color).changed() {
                shape.stroke.r = stroke_color.r();
                shape.stroke.g = stroke_color.g();
                shape.stroke.b = stroke_color.b();
                changed = true;
            }
        });
        changed |= ui
            .add(
                egui::DragValue::new(&mut shape.stroke_width)
                    .prefix("Толщина обводки: ")
                    .speed(0.1)
                    .range(0.0..=20.0),
            )
            .changed();

        ui.separator();
        ui.label("Скругление углов:");
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
            changed = true;
        }
        ui.horizontal(|ui| {
            ui.label("Левый верхний:");
            changed |= ui
                .add(
                    egui::DragValue::new(&mut shape.corner_tl)
                        .speed(0.5)
                        .range(0.0..=100.0),
                )
                .changed();
        });
        ui.horizontal(|ui| {
            ui.label("Правый верхний:");
            changed |= ui
                .add(
                    egui::DragValue::new(&mut shape.corner_tr)
                        .speed(0.5)
                        .range(0.0..=100.0),
                )
                .changed();
        });
        ui.horizontal(|ui| {
            ui.label("Левый нижний:");
            changed |= ui
                .add(
                    egui::DragValue::new(&mut shape.corner_bl)
                        .speed(0.5)
                        .range(0.0..=100.0),
                )
                .changed();
        });
        ui.horizontal(|ui| {
            ui.label("Правый нижний:");
            changed |= ui
                .add(
                    egui::DragValue::new(&mut shape.corner_br)
                        .speed(0.5)
                        .range(0.0..=100.0),
                )
                .changed();
        });

        if shape.tool == DrawTool::Text {
            ui.separator();
            ui.label("Текст:");
            changed |= ui.text_edit_singleline(&mut shape.text).changed();
            changed |= ui
                .add(
                    egui::DragValue::new(&mut shape.font_size)
                        .prefix("Размер шрифта: ")
                        .speed(0.5)
                        .range(4.0..=200.0),
                )
                .changed();
            ui.horizontal(|ui| {
                ui.label("Цвет текста:");
                let mut text_color = Color32::from_rgb(
                    shape.text_color.r,
                    shape.text_color.g,
                    shape.text_color.b,
                );
                if ui.color_edit_button_srgba(&mut text_color).changed() {
                    shape.text_color.r = text_color.r();
                    shape.text_color.g = text_color.g();
                    shape.text_color.b = text_color.b();
                    changed = true;
                }
            });
            ui.label("Выравнивание:");
            let mut align = shape.text_align.clone();
            egui::ComboBox::from_label("")
                .selected_text(match align {
                    TextAlign::Left => "По левому",
                    TextAlign::Center => "По центру",
                    TextAlign::Right => "По правому",
                })
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut align, TextAlign::Left, "По левому").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut align, TextAlign::Center, "По центру").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut align, TextAlign::Right, "По правому").changed() {
                        changed = true;
                    }
                });
            shape.text_align = align;
        }

        ui.separator();
        if ui.button("Удалить фигуру").clicked() {
            delete = true;
        }

        (changed, delete)
    }

    fn canvas_panel(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size();
        let canvas_rect = egui::Rect::from_min_size(ui.cursor().min, available);

        // Обработка ввода сначала (избегаем конфликта заимствований с painter)
        let response = ui.allocate_rect(canvas_rect, Sense::click_and_drag());
        let painter = ui.painter();

        let middle_down = ui.ctx().input(|i| i.pointer.button_down(PointerButton::Middle));

        // Приближение/отдаление колёсиком мыши к курсору.
        // Скролл вниз (от себя) — отдаление, скролл вверх (на себя) — приближение.
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

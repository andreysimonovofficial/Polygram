//! Figma-подобный свободный холст: рисуем прямоугольники, линии, эллипсы, текст,
//! перетаскиваем элементы, магнитное выравнивание, панорамирование, зум колёсиком,
//! автосохранение последнего проекта и двусторонний редактор кода для Агента.

use crate::design_format::*;
use crate::input::ResizeMode;
use eframe::egui;
use egui::{Key, Pos2, Vec2};
use std::collections::HashSet;

/// Состояние свободного холста.
pub struct FreeCanvas {
    pub doc: FreeCanvasDoc,
    pub selected: Vec<usize>,
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
    /// Режим прототипа (синхронизируется с doc.prototype_mode).
    pub prototype_mode: bool,
    /// Прямая цель наведения (topmost hit под курсором).
    pub direct_hover: Option<usize>,
    /// Прямая цель нажатия.
    pub direct_press: Option<usize>,
    /// Активное множество наведения: цель + дети + группа.
    pub hovered: HashSet<usize>,
    /// Активное множество нажатия: цель + дети + группа.
    pub pressed: HashSet<usize>,
    /// Контекстное меню: позиция на экране и опциональный индекс целевой фигуры.
    pub context_menu: Option<(Pos2, Option<usize>)>,
    /// Экранный прямоугольник selection box (рисуется в конце canvas_panel, верхний слой).

    /// Стек состояний для Undo (состояние ДО каждого действия).
    pub undo_stack: Vec<FreeCanvasDoc>,
    /// Стек состояний для Redo (состояние ПОСЛЕ отменённых действий).
    pub redo_stack: Vec<FreeCanvasDoc>,
    /// Максимальный размер стеков.
    pub max_history: usize,
    /// Буфер обмена (JSON-строка скопированных фигур).
    pub clipboard: Option<String>,
    /// Флаг: история уже сохранена для текущего перетаскивания.
    pub drag_history_saved: bool,
    /// Режим изменения размера (какой угол/сторона захвачен).
    pub resize_mode: Option<ResizeMode>,
    /// Исходное состояние выделенных фигур перед началом ресайза: (индекс, x, y, w, h) для каждой.
    pub resize_origin: Vec<(usize, f32, f32, f32, f32)>,
    /// Зафиксированная магнитная цель ресайза по осям (гистерезис), чтобы край не дёргался
    /// на границе порога и сцепление было воспроизводимо при повторном захвате ручки.
    pub resize_mag_locked_x: Option<f32>,
    pub resize_mag_locked_y: Option<f32>,
}

impl Default for FreeCanvas {
    fn default() -> Self {
        Self {
            doc: FreeCanvasDoc::default(),
            selected: Vec::new(),
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
            prototype_mode: false,
            direct_hover: None,
            direct_press: None,
            hovered: HashSet::new(),
            pressed: HashSet::new(),
            context_menu: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history: 50,
            clipboard: None,
            drag_history_saved: false,
            resize_mode: None,
            resize_origin: Vec::new(),
            resize_mag_locked_x: None,
            resize_mag_locked_y: None,
        }
    }
}

impl FreeCanvas {
    pub fn new() -> Self {
        Self::default()
    }

    /// Сохранить заданное состояние документа в стек Undo (состояние ДО изменения).
    /// Вызывать перед применением изменения, когда нужно сохранить конкретное состояние.
    pub fn push_undo_state(&mut self, doc: FreeCanvasDoc) {
        if self.undo_stack.len() >= self.max_history {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(doc);
        self.redo_stack.clear();
    }

    /// Сохранить текущее состояние документа в стек Undo (состояние ДО изменения).
    /// Вызывать перед применением изменения к документу.
    pub fn save_undo_state(&mut self) {
        if self.undo_stack.len() >= self.max_history {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(self.doc.clone());
        self.redo_stack.clear();
    }

    /// Можно ли отменить последнее изменение.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Можно ли повторить отменённое изменение.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Отменить последнее изменение.
    pub fn undo(&mut self) {
        if self.undo_stack.is_empty() {
            return;
        }
        // Сохраняем текущее состояние в стек Redo
        self.redo_stack.push(self.doc.clone());
        // Восстанавливаем состояние из Undo
        self.doc = self.undo_stack.pop().unwrap();
        self.recompute_next_id();
        self.selected.clear();
        self.context_menu = None;
        self.hovered.clear();
        self.pressed.clear();
        self.direct_hover = None;
        self.direct_press = None;
        self.touch();
    }

    /// Повторить отменённое изменение.
    pub fn redo(&mut self) {
        if self.redo_stack.is_empty() {
            return;
        }
        // Сохраняем текущее состояние в стек Undo
        self.undo_stack.push(self.doc.clone());
        // Восстанавливаем состояние из Redo
        self.doc = self.redo_stack.pop().unwrap();
        self.recompute_next_id();
        self.selected.clear();
        self.context_menu = None;
        self.hovered.clear();
        self.pressed.clear();
        self.direct_hover = None;
        self.direct_press = None;
        self.touch();
    }

    /// Скопировать выделенные фигуры в буфер обмена (JSON).
    pub fn copy_selected(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        let selected_shapes: Vec<&FreeShape> = self.selected.iter()
            .filter_map(|&i| self.doc.shapes.get(i))
            .collect();
        if let Ok(json) = serde_json::to_string(&selected_shapes) {
            self.clipboard = Some(json);
        }
    }

    /// Вставить фигуры из буфера обмена.
    pub fn paste_from_clipboard(&mut self) {
        self.save_undo_state();
        let Some(json) = &self.clipboard else { return };
        let Ok(shapes): Result<Vec<FreeShape>, _> = serde_json::from_str(json) else { return };
        let shapes_len = shapes.len();
        for shape in &shapes {
            let mut new_shape = shape.clone();
            new_shape.id = format!("shape-{}", self.next_id);
            self.next_id += 1;
            // Смещаем вставленные фигуры чуть вправо-вниз, чтобы они не лежали точно поверх исходных
            new_shape.x += 20.0;
            new_shape.y += 20.0;
            self.doc.shapes.push(new_shape);
        }
        self.selected.clear();
        for i in (self.doc.shapes.len() - shapes_len)..self.doc.shapes.len() {
            self.selected.push(i);
        }
        self.touch();
    }

    /// Отобразить нижнюю панель инструментов, правую боковую панель свойств,
    /// левую боковую панель кода и холст.
    pub fn ui(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Удаление выделенного по клавише Delete
        if ctx.input(|i| i.key_pressed(Key::Delete)) {
            self.delete_selected();
            self.touch();
        }

        // Горячие клавиши Undo/Redo
        let (ctrl, z_pressed, y_pressed) = ctx.input(|i| (i.modifiers.ctrl, i.key_pressed(Key::Z), i.key_pressed(Key::Y)));
        if ctrl && z_pressed {
            self.undo();
        }
        if ctrl && y_pressed {
            self.redo();
        }

        // Загрузка настроек при первом кадре
        if !self.settings_loaded {
            if let Some(storage) = frame.storage() {
                self.load_settings(storage);
            }
            self.settings_loaded = true;
        }

        // Синхронизировать режим прототипа с документом.
        self.prototype_mode = self.doc.prototype_mode;

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
                                self.drag_start = None;
                                self.drag_shape_pos = None;
                            }
                        }
                    });
                    ui.separator();
                    if ui.button("Код для агента").clicked() {
                        self.show_code_sidebar = !self.show_code_sidebar;
                        if self.show_code_sidebar {
                            self.code_text = self.doc.to_json().unwrap_or_default();
                            self.last_applied_code = self.code_text.clone();
                        }
                    }
                    ui.separator();
                    let proto_label = if self.prototype_mode { "Прототип: ВКЛ" } else { "Прототип: ВЫКЛ" };
                    if ui.button(proto_label).clicked() {
                        self.doc.prototype_mode = !self.doc.prototype_mode;
                        self.prototype_mode = self.doc.prototype_mode;
                        self.hovered.clear();
                        self.pressed.clear();
                    }
                    ui.separator();
                    if ui.button("↶ Отменить (Ctrl+Z)").clicked() && self.can_undo() {
                        self.undo();
                    }
                    if ui.button("↷ Повторить (Ctrl+Y)").clicked() && self.can_redo() {
                        self.redo();
                    }
                    ui.separator();
                    let has_sel = !self.selected.is_empty();
                    if ui.button("На передний план").clicked() && has_sel {
                        self.send_to_front();
                    }
                    if ui.button("На задний план").clicked() && has_sel {
                        self.send_to_back();
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

                    // Если дизайн изменился через UI и пользователь не правит код вручную,
                    // синхронизируем код в поле (дизайн → код).
                    let current_json = self.doc.to_json().unwrap_or_default();
                    if self.code_text == self.last_applied_code && self.code_text != current_json {
                        self.code_text = current_json.clone();
                        self.last_applied_code = current_json;
                    }

                    // Поле кода с вертикальным скроллом, ограниченное по высоте экрана.
                    egui::ScrollArea::vertical()
                        .max_height(ui.available_height())
                        .show(ui, |ui| {
                            let available_width = ui.available_width();
                            egui::TextEdit::multiline(&mut self.code_text)
                                .desired_width(available_width)
                                .show(ui);
                        });

                    // Применяем изменения из кода в дизайн «на лету» (код → дизайн),
                    // но только когда текст реально изменился, чтобы не сбрасывать курсор.
                    if self.code_text != self.last_applied_code {
                        if let Ok(new_doc) = FreeCanvasDoc::load_from_json(&self.code_text) {
                            self.doc = new_doc;
                            self.recompute_next_id();
                            self.selected.clear();
                            self.touch();
                            self.last_applied_code = self.code_text.clone();
                        }
                    }
                });
        }

        // Правая боковая панель: свойства
        let response = egui::SidePanel::right("properties")
            .resizable(true)
            .default_width(self.sidebar_width)
            .min_width(200.0)
            .max_width(1000.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        self.properties_panel(ui);
                    });
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
}

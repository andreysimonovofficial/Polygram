//! Figma-подобный свободный холст: рисуем прямоугольники, линии, эллипсы, текст,
//! перетаскиваем элементы, магнитное выравнивание, панорамирование, зум колёсиком,
//! автосохранение последнего проекта и двусторонний редактор кода для Агента.

use crate::design_format::*;
use eframe::egui;
use egui::{Key, Pos2, Vec2};

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
    /// Режим прототипа (синхронизируется с doc.prototype_mode).
    pub prototype_mode: bool,
    /// Индекс фигуры под курсором (только в режиме прототипа).
    pub hovered: Option<usize>,
    /// Индекс зажатой фигуры (только в режиме прототипа).
    pub pressed: Option<usize>,
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
            prototype_mode: false,
            hovered: None,
            pressed: None,
        }
    }
}

impl FreeCanvas {
    pub fn new() -> Self {
        Self::default()
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
                        self.hovered = None;
                        self.pressed = None;
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
                            self.selected = None;
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
}

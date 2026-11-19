//! Состояние приложения Polygram.
//! Хранит Canvas, отслеживает клики по холсту, выделение объектов,
//! перетаскивание отделов за шапку и диалог переименования.
//! Также отвечает за установку загрузчиков изображений egui_extras (включая SVG).

use egui::{Pos2, PointerButton};
use polygram_canvas::{Canvas, ObjectKind};

mod icons;
mod interaction;

/// Главное состояние приложения Polygram.
pub struct PolygramApp {
    canvas: Canvas,
    /// Экранная позиция последнего правого клика по холсте (открывает контекстное меню).
    right_click: Option<Pos2>,
    /// Объект, на который был сделан правый клик (если отдел).
    right_click_object: Option<String>,
    /// Установлены ли загрузчики изображений egui_extras (включая SVG-загрузчик).
    image_loaders_installed: bool,
    /// Меню только что открыто правым кликом — игнорировать close-логику в этом кадре.
    menu_just_opened: bool,
    /// Выделенные объекты.
    pub selected_objects: Vec<String>,
    /// Объект, который сейчас перетаскивается (None — не перетаскивается).
    drag_object: Option<String>,
    /// Смещение курсора относительно левого верхнего угла отдела в клетках.
    drag_offset_cell: (f32, f32),
    /// Начальная позиция мыши при начале драга (экран).
    drag_start_screen: Option<Pos2>,
    /// Объект, который сейчас переименовывается.
    pub rename_target: Option<String>,
    /// Введённое новое имя для переименования.
    pub rename_input: String,
}

impl PolygramApp {
    /// Создаёт новое состояние приложения с холстом по умолчанию.
    pub fn new() -> Self {
        Self {
            canvas: Canvas::new(),
            right_click: None,
            right_click_object: None,
            image_loaders_installed: false,
            menu_just_opened: false,
            selected_objects: Vec::new(),
            drag_object: None,
            drag_offset_cell: (0.0, 0.0),
            drag_start_screen: None,
            rename_target: None,
            rename_input: String::new(),
        }
    }

    /// Найти объект (отдел) под заданной экранной позицией.
    /// Возвращает id отдела, если позиция попадает в rect отдела.
    fn object_under(&self, screen_pos: Pos2) -> Option<String> {
        let cell_px = self.canvas.cell_px();
        for object in &self.canvas.objects {
            if object.kind != ObjectKind::Department {
                continue;
            }
            let r = &object.rect;
            let rect_px = egui::Rect::from_min_size(
                self.canvas.cell_to_screen(r.x, r.y),
                egui::Vec2::new(r.w as f32 * cell_px, r.h as f32 * cell_px),
            );
            if rect_px.contains(screen_pos) {
                return Some(object.id.clone());
            }
        }
        None
    }

    /// Найти отдел, чья шапка содержит заданную экранную позицию.
    fn header_under(&self, screen_pos: Pos2) -> Option<String> {
        let cell_px = self.canvas.cell_px();
        for object in &self.canvas.objects {
            if object.kind != ObjectKind::Department {
                continue;
            }
            let r = &object.rect;
            let rect_px = egui::Rect::from_min_size(
                self.canvas.cell_to_screen(r.x, r.y),
                egui::Vec2::new(r.w as f32 * cell_px, r.h as f32 * cell_px),
            );
            let header_h = cell_px * 1.0;
            let header_rect =
                egui::Rect::from_min_size(rect_px.min, egui::Vec2::new(rect_px.width(), header_h));
            if header_rect.contains(screen_pos) {
                return Some(object.id.clone());
            }
        }
        None
    }

    /// Снять выделение со всех объектов.
    pub fn clear_selection(&mut self) {
        self.selected_objects.clear();
    }

    /// Выделить объект (добавить, если уже выделен — убрать при Ctrl).
    pub fn toggle_select(&mut self, id: String, ctrl: bool) {
        if ctrl {
            if self.selected_objects.iter().any(|s| s == &id) {
                self.selected_objects.retain(|s| s != &id);
            } else {
                self.selected_objects.push(id);
            }
        } else {
            self.selected_objects = vec![id];
        }
    }
}

impl Default for PolygramApp {
    fn default() -> Self {
        Self::new()
    }
}

impl eframe::App for PolygramApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Устанавливаем загрузчики изображений egui_extras (включая SVG) при первом кадре.
        if !self.image_loaders_installed {
            egui_extras::loaders::install_image_loaders(ui.ctx());
            self.image_loaders_installed = true;
        }

        egui::CentralPanel::default().show(ui, |ui| {
            self.canvas.paint(ui, &self.selected_objects);

            let canvas_response = ui.interact(
                ui.clip_rect(),
                ui.scope_id().with("canvas"),
                egui::Sense::click(),
            );

            // ---- ЛКМ клик: выделение или начало драга ----
            if canvas_response.clicked() {
                    if let Some(pos) = canvas_response.interact_pointer_pos() {
                        let ctrl = ui.ctx().input(|i| i.modifiers.ctrl);
                        if let Some(id) = self.header_under(pos) {
                            // Клик на шапку отдела — выделяем отдел (не начинаем драг здесь).
                            self.toggle_select(id.clone(), ctrl);
                            self.drag_start_screen = Some(pos);
                            if let Some(obj) = self.canvas.objects.iter().find(|o| o.id == id) {
                                let screen_min = self.canvas.cell_to_screen(obj.rect.x, obj.rect.y);
                                let click_cell_x = ((pos.x / self.canvas.camera.zoom
                                    - self.canvas.camera.offset.x)
                                    / self.canvas.cell_size) as f32;
                                let click_cell_y = ((pos.y / self.canvas.camera.zoom
                                    - self.canvas.camera.offset.y)
                                    / self.canvas.cell_size) as f32;
                                let top_left_cell_x =
                                    (screen_min.x / self.canvas.camera.zoom
                                        - self.canvas.camera.offset.x)
                                    / self.canvas.cell_size;
                                let top_left_cell_y =
                                    (screen_min.y / self.canvas.camera.zoom
                                        - self.canvas.camera.offset.y)
                                    / self.canvas.cell_size;
                                self.drag_offset_cell =
                                    (click_cell_x - top_left_cell_x, click_cell_y - top_left_cell_y);
                            }
                        } else if let Some(id) = self.object_under(pos) {
                        // Клик на тело отдела — выделяем.
                        self.toggle_select(id, ctrl);
                        self.drag_start_screen = None;
                    } else {
                        // Клик по пустому месту — снимаем выделение (кроме Ctrl).
                        if !ctrl {
                            self.clear_selection();
                        }
                        self.drag_start_screen = None;
                    }
                }
            }

            // ---- Начало перетаскивания отдела за шапку ----
            if canvas_response.drag_started_by(PointerButton::Primary)
                && self.drag_object.is_none()
            {
                if let Some(pos) = canvas_response.interact_pointer_pos() {
                    if let Some(id) = self.header_under(pos) {
                        self.drag_object = Some(id.clone());
                        self.selected_objects = vec![id];
                        if let Some(obj) = self
                            .canvas
                            .objects
                            .iter()
                            .find(|o| o.id == self.drag_object.as_deref().unwrap())
                        {
                            let screen_min = self.canvas.cell_to_screen(obj.rect.x, obj.rect.y);
                            let click_cell_x = ((pos.x / self.canvas.camera.zoom
                                - self.canvas.camera.offset.x)
                                / self.canvas.cell_size) as f32;
                            let click_cell_y = ((pos.y / self.canvas.camera.zoom
                                - self.canvas.camera.offset.y)
                                / self.canvas.cell_size) as f32;
                            let top_left_cell_x =
                                (screen_min.x / self.canvas.camera.zoom
                                    - self.canvas.camera.offset.x)
                                / self.canvas.cell_size;
                            let top_left_cell_y =
                                (screen_min.y / self.canvas.camera.zoom
                                    - self.canvas.camera.offset.y)
                                / self.canvas.cell_size;
                            self.drag_offset_cell =
                                (click_cell_x - top_left_cell_x, click_cell_y - top_left_cell_y);
                        }
                    }
                }
            }

            // ---- Обновление позиции при перетаскивании ----
            if let Some(ref drag_id) = self.drag_object {
                if canvas_response.dragged_by(PointerButton::Primary) {
                    if let Some(cur) = canvas_response.interact_pointer_pos() {
                        let cursor_world_x =
                            (cur.x / self.canvas.camera.zoom - self.canvas.camera.offset.x)
                                / self.canvas.cell_size;
                        let cursor_world_y =
                            (cur.y / self.canvas.camera.zoom - self.canvas.camera.offset.y)
                                / self.canvas.cell_size;
                        let target_x = (cursor_world_x - self.drag_offset_cell.0).round() as i32;
                        let target_y = (cursor_world_y - self.drag_offset_cell.1).round() as i32;
                        if let Some(obj) = self
                            .canvas
                            .objects
                            .iter_mut()
                            .find(|o| o.id == *drag_id)
                        {
                            obj.rect.x = target_x;
                            obj.rect.y = target_y;
                        }
                    }
                }
            }

            // ---- Завершение перетаскивания ----
            if self.drag_object.is_some()
                && !ui.input(|i| i.pointer.button_down(PointerButton::Primary))
            {
                self.drag_object = None;
                self.drag_start_screen = None;
            }

            // ---- ПКМ: запомнить объект под курсором для контекстного меню ----
            if canvas_response.secondary_clicked() {
                if let Some(pos) = canvas_response.interact_pointer_pos() {
                    self.right_click = Some(pos);
                    self.right_click_object = self.object_under(pos);
                    self.menu_just_opened = true;
                }
            }
        });

        interaction::context_menu(self, ui.ctx());
        interaction::rename_dialog(self, ui.ctx());

        ui.ctx().request_repaint();
    }
}

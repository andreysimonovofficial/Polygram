//! Состояние приложения Polygram.
//! Хранит Canvas, движок ядра, отслеживает клики по холсту, выделение объектов,
//! перетаскивание отделов за шапку, диалог переименования и диалоги показателей.

use egui::{Pos2, PointerButton, Rect, Vec2};
use polygram_canvas::Canvas;
use polygram_core::{Command, Engine, runtime::Published, types::{Direction, NodeKind}};
use polygram_core::drag_domain::{compute_drag_preview, DragPreviewInput};
use polygram_core::geometry::Geometry;
use polygram_core::layout::Rect as CoreRect;

mod dialogs;
mod icons;
mod interaction;

/// Главное состояние приложения Polygram.
pub struct PolygramApp {
    canvas: Canvas,
    /// Движок ядра Polygram (иерархия, значения, статусы, команды).
    engine: Engine,
    /// Экранная позиция последнего правого клика по холсте (открывает контекстное меню).
    right_click: Option<Pos2>,
    /// Объект, на который был сделан правый клик (если отдел или процесс).
    right_click_object: Option<String>,
    /// Установлены ли загрузчики изображений egui_extras (включая SVG).
    image_loaders_installed: bool,
    /// Меню только что открыто правым кликом — игнорировать close-логику в этом кадре.
    menu_just_opened: bool,
    /// Выделенные объекты.
    pub selected_objects: Vec<String>,
    /// Объект, который сейчас перетаскивается (None — не перетаскивается).
    drag_object: Option<String>,
    /// Смещение курсора относительно левого верхнего угла объекта в клетках.
    drag_offset_cell: (f32, f32),
    /// Начальная позиция мыши при начале драга (экран).
    drag_start_screen: Option<Pos2>,
    /// Стартовая клетка перетаскиваемого объекта (для отмены при возврате на место).
    drag_start_cell: Option<(i32, i32)>,
    /// Отдел, над которым нависает перетаскиваемый объект.
    drag_over: Option<String>,
    /// Свободная клетка для призрачной копии при перетаскивании процесса.
    drag_ghost: Option<(i32, i32)>,
    /// Источник перетаскивания показателя: (id объекта, направление, имя показателя).
    drag_metric_source: Option<(String, Direction, String)>,
    /// Начальная позиция перетаскивания показателя (экран).
    drag_metric_start: Option<Pos2>,
    /// Объект, который сейчас переименовывается.
    pub rename_target: Option<String>,
    /// Введённое новое имя для переименования.
    pub rename_input: String,
    /// Стартовая позиция выделения рамкой (selection box), если активна.
    pub selection_box_start: Option<Pos2>,
    /// Состояние диалогов (переименование, добавление показателя, контейнер и др.).
    pub dialogs: dialogs::DialogState,
}

impl PolygramApp {
    /// Создаёт новое состояние приложения с холстом и движком по умолчанию.
    pub fn new() -> Self {
        Self {
            canvas: Canvas::new(),
            engine: Engine::new(),
            right_click: None,
            right_click_object: None,
            image_loaders_installed: false,
            menu_just_opened: false,
            selected_objects: Vec::new(),
            drag_object: None,
            drag_offset_cell: (0.0, 0.0),
            drag_start_screen: None,
            drag_start_cell: None,
            drag_over: None,
            drag_ghost: None,
            drag_metric_source: None,
            drag_metric_start: None,
            rename_target: None,
            rename_input: String::new(),
            selection_box_start: None,
            dialogs: dialogs::DialogState::new(),
        }
    }

    /// Выполнить команду в движке и обновить опубликованное состояние.
    pub fn apply_cmd(&mut self, cmd: Command) {
        let _ = self.engine.apply(polygram_core::types::Scope::Root, cmd);
        self.sync_canvas_from_engine();
    }

    /// Выполнить пачку команд в движке.
    pub fn apply_batch(&mut self, cmds: Vec<Command>) {
        let _ = self.engine.apply_batch(polygram_core::types::Scope::Root, cmds);
        self.sync_canvas_from_engine();
    }

    /// Синхронизировать объекты Canvas с геометрией движка (после применения команд).
    fn sync_canvas_from_engine(&mut self) {
        let geometry = self.engine.geometry();
        self.canvas.objects = geometry.objects;
    }

    /// Последний опубликованный снимок состояния движка.
    pub fn published(&self) -> Published {
        let geometry = self.engine.geometry();
        let dynamic = self.engine.dynamic_state();
        let metric_defs = self.engine.metric_def_views();
        Published {
            rev: self.engine.rev(),
            dynamic,
            geometry,
            metric_defs,
            camera_fit_requested: self.engine.camera_fit_requested(),
            auto_screenshot: self.engine.auto_screenshot(),
        }
    }

    /// Найти отдел под заданной экранной позицией (внутри rect отдела).
    fn object_under(&self, screen_pos: Pos2) -> Option<String> {
        let cell_px = self.canvas.cell_px();
        for object in &self.canvas.objects {
            if object.kind != NodeKind::Department {
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
            if object.kind != NodeKind::Department {
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

    /// Начать перетаскивание отдела за шапку.
    fn start_drag_object(&mut self, id: String, screen_pos: Pos2) {
        self.drag_object = Some(id.clone());
        self.selected_objects = vec![id.clone()];
        if let Some(obj) = self.canvas.objects.iter().find(|o| o.id == id) {
            let screen_min = self.canvas.cell_to_screen(obj.rect.x, obj.rect.y);
            let click_cell_x = ((screen_pos.x / self.canvas.camera.zoom
                - self.canvas.camera.offset.x)
                / self.canvas.cell_size) as f32;
            let click_cell_y = ((screen_pos.y / self.canvas.camera.zoom
                - self.canvas.camera.offset.y)
                / self.canvas.cell_size) as f32;
            let top_left_cell_x =
                (screen_min.x / self.canvas.camera.zoom - self.canvas.camera.offset.x)
                    / self.canvas.cell_size;
            let top_left_cell_y =
                (screen_min.y / self.canvas.camera.zoom - self.canvas.camera.offset.y)
                    / self.canvas.cell_size;
            self.drag_offset_cell = (
                click_cell_x - top_left_cell_x,
                click_cell_y - top_left_cell_y,
            );
        }
        self.drag_start_screen = Some(screen_pos);
    }

    /// Применить текущую позицию курсора к перетаскиваемому отделу (привязка к клеткам).
    fn update_drag_object(&mut self, cursor: Pos2) {
        let Some(drag_id) = self.drag_object.as_deref() else {
            return;
        };
        let cursor_world_x = (cursor.x / self.canvas.camera.zoom - self.canvas.camera.offset.x)
            / self.canvas.cell_size;
        let cursor_world_y = (cursor.y / self.canvas.camera.zoom - self.canvas.camera.offset.y)
            / self.canvas.cell_size;
        let target_x = (cursor_world_x - self.drag_offset_cell.0).round() as i32;
        let target_y = (cursor_world_y - self.drag_offset_cell.1).round() as i32;
        if let Some(obj) = self.canvas.objects.iter_mut().find(|o| o.id == drag_id) {
            obj.rect.x = target_x;
            obj.rect.y = target_y;
        }
    }

    /// Завершить перетаскивание отдела.
    fn end_drag_object(&mut self) {
        self.drag_object = None;
        self.drag_start_screen = None;
    }

    /// Найти горячую зону показателя под заданной экранной позицией.
    fn drag_metric_under(&self, pos: Pos2) -> Option<(String, Direction, String)> {
        for (id, dir, name, rect) in &self.canvas.drag_metric_hotspots {
            if rect.contains(pos) {
                return Some((id.clone(), *dir, name.clone()));
            }
        }
        None
    }

    /// Найти процесс под экранной позицией (динамическая CSS-геометрия).
    fn process_under(&self, pos: Pos2, dynamic: &polygram_core::DynamicState) -> Option<String> {
        let zoom = self.canvas.camera.zoom;
        for object in &self.canvas.objects {
            if object.kind != NodeKind::Process {
                continue;
            }
            let r = &object.rect;
            let card_w = 512.0 * zoom;
            let card_h = polygram_canvas::paint::object::process_total_height(object, dynamic, zoom, card_w);
            let rect_px = Rect::from_min_size(
                self.canvas.cell_to_screen(r.x, r.y),
                Vec2::new(card_w, card_h),
            );
            if rect_px.contains(pos) {
                return Some(object.id.clone());
            }
        }
        None
    }

    /// Начать перетаскивание процесса.
    fn start_drag_process(&mut self, id: String, screen_pos: Pos2) {
        if let Some(obj) = self.canvas.objects.iter().find(|o| o.id == id) {
            self.drag_start_cell = Some((obj.rect.x, obj.rect.y));
        }
        self.start_drag_object(id.clone(), screen_pos);
        self.drag_over = None;
        self.drag_ghost = None;
    }

    /// Вычислить превью перетаскивания процесса: отдел под курсором и свободная клетка для призрака.
    fn process_drag_preview(&self, cursor: Pos2) -> (Option<String>, Option<(i32, i32)>) {
        let Some(drag_id) = self.drag_object.as_deref() else {
            return (None, None);
        };
        let dragged_obj = match self.canvas.objects.iter().find(|o| o.id == drag_id) {
            Some(o) => o.clone(),
            None => return (None, None),
        };
        let zoom = self.canvas.camera.zoom;
        let offset = self.canvas.camera.offset;
        let cell_size = self.canvas.cell_size;
        let cursor_world_x = (cursor.x / zoom - offset.x) / cell_size;
        let cursor_world_y = (cursor.y / zoom - offset.y) / cell_size;
        let cursor_cell = (cursor_world_x.round() as i32, cursor_world_y.round() as i32);

        let geometry = Geometry {
            rev: 0,
            objects: self.canvas.objects.clone(),
            edges: Vec::new(),
        };
        let preview = compute_drag_preview(&DragPreviewInput {
            geometry,
            dragged_id: drag_id.to_owned(),
            dragged_size: (16, 16),
            cursor_cell,
        });
        (preview.over, preview.ghost)
    }

    /// Обновить позицию перетаскиваемого процесса, нарисовать призрачную копию и подсветку отдела.
    fn update_drag_process(&mut self, ui: &mut egui::Ui, cursor: Pos2, theme: &polygram_canvas::Theme) {
        self.update_drag_object(cursor);
        let (over, ghost) = self.process_drag_preview(cursor);
        self.drag_over = over.clone();
        self.drag_ghost = ghost;

        let painter = ui.painter();
        let cell_px = self.canvas.cell_px();
        let zoom = self.canvas.camera.zoom;
        let offset = self.canvas.camera.offset;

        // Подсветить отдел, над которым нависает процесс.
        if let Some(dept_id) = &over {
            if let Some(dept) = self.canvas.objects.iter().find(|o| o.id == *dept_id && o.kind == NodeKind::Department)
            {
                let r = &dept.rect;
                let rect_px = Rect::from_min_size(
                    self.canvas.cell_to_screen(r.x, r.y),
                    Vec2::new(r.w as f32 * cell_px, r.h as f32 * cell_px),
                );
                painter.rect_stroke(
                    rect_px.expand(2.0),
                    2.0,
                    egui::Stroke::new(2.0 * zoom, theme.accent),
                    egui::StrokeKind::Middle,
                );
            }
        }

        // Нарисовать пустую пунктирную рамку ("призрачную копию") в целевой клетке.
        if let Some((gx, gy)) = ghost {
            let rect_px = Rect::from_min_size(
                Pos2::new(gx as f32, gy as f32) * cell_px + offset * zoom,
                Vec2::new(16.0 * cell_px, 16.0 * cell_px),
            );
            polygram_canvas::paint_dashed_rect(painter, rect_px, zoom, theme.accent);
        }
    }

    /// Завершить перетаскивание процесса: перенос в отдел (с подтверждением) или обычное перемещение.
    fn end_drag_process(&mut self) {
        let Some(drag_id) = self.drag_object.clone() else {
            return;
        };
        let start_cell = self.drag_start_cell.unwrap_or((0, 0));
        let Some(obj) = self.canvas.objects.iter().find(|o| o.id == drag_id).cloned() else {
            self.drag_object = None;
            self.drag_start_screen = None;
            self.drag_start_cell = None;
            self.drag_over = None;
            self.drag_ghost = None;
            return;
        };

        // Конечная клетка: призрачная копия, если есть, иначе текущая позиция объекта.
        let drop_cell = self.drag_ghost.unwrap_or((obj.rect.x, obj.rect.y));

        // Если процесс над отделом — открыть модальное подтверждение переноса.
        if let Some(dept_id) = &self.drag_over {
            self.dialogs.reparent_confirm = Some((
                drag_id.clone(),
                dept_id.clone(),
                drop_cell,
                start_cell,
                NodeKind::Process,
            ));
            self.drag_object = None;
            self.drag_start_screen = None;
            self.drag_start_cell = None;
            self.drag_over = None;
            self.drag_ghost = None;
            return;
        }

        // Иначе — обычное перемещение, если позиция изменилась.
        if drop_cell != start_cell {
            self.apply_batch(vec![Command::MoveObject {
                object: drag_id.clone(),
                at: [drop_cell.0, drop_cell.1],
            }]);
        } else {
            // Вернуть на исходную позицию (отмена).
            if let Some(o) = self.canvas.objects.iter_mut().find(|o| o.id == drag_id) {
                o.rect.x = start_cell.0;
                o.rect.y = start_cell.1;
            }
        }

        self.drag_object = None;
        self.drag_start_screen = None;
        self.drag_start_cell = None;
        self.drag_over = None;
        self.drag_ghost = None;
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

        let published = self.published();

        // Если открыт полноэкранный экран показателей — рисуем его вместо холста.
        if self.dialogs.show_metrics_screen {
                interaction::container_menu::render_metrics_screen(
                    self,
                    ui,
                    &published,
                    self.canvas.theme,
                );
            ui.ctx().request_repaint();
            return;
        }

        egui::CentralPanel::default().show(ui, |ui| {
            let canvas_response = ui.interact(
                ui.clip_rect(),
                ui.scope_id().with("canvas"),
                egui::Sense::click_and_drag(),
            );

            self.canvas.handle_camera(ui.ctx(), &canvas_response, ui);

            self.canvas.paint(
                ui,
                &published.dynamic,
                self.right_click_object.as_deref(),
                &self.selected_objects,
            );

            if !canvas_response.hovered() {
                interaction::context_menu(self, ui.ctx(), &published, self.canvas.theme);
                interaction::rename_dialog(self, ui.ctx());
                ui.ctx().request_repaint();
                return;
            }

            // ---- ПКМ: контекстное меню на отделе/процессе или пустом месте ----
            if canvas_response.secondary_clicked() {
                if let Some(pos) = canvas_response.interact_pointer_pos() {
                    self.selection_box_start = None;
                    self.right_click = Some(pos);
                    self.right_click_object = self.object_under(pos);
                    if self.right_click_object.is_none() {
                        // Проверяем, не попали ли в процесс (вне отделов).
                        if let Some(obj_id) = self.process_under(pos, &published.dynamic) {
                            self.right_click_object = Some(obj_id);
                        }
                    }
                    self.menu_just_opened = true;
                }
            }

            // ---- ЛКМ клик: выделение объекта ----
            if canvas_response.clicked() {
                if let Some(pos) = canvas_response.interact_pointer_pos() {
                    let ctrl = ui.ctx().input(|i| i.modifiers.ctrl);
                    if let Some(id) = self.header_under(pos) {
                        self.selection_box_start = None;
                        self.toggle_select(id.clone(), ctrl);
                    } else {
                        // Проверяем клик по процессу.
                        let mut clicked_process = false;
                        if let Some(obj_id) = self.process_under(pos, &published.dynamic) {
                            self.selection_box_start = None;
                            self.toggle_select(obj_id.clone(), ctrl);
                            clicked_process = true;
                        }
                        if !clicked_process {
                            // Клик по пустому месту — ничего не выделяем (или можно сбрасывать выделение).
                        }
                    }
                }
            }

            // ---- Двойной клик по процессу — инлайн-переименование ----
            if canvas_response.hovered()
                && ui.ctx().input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary))
            {
                if let Some(pos) = canvas_response.interact_pointer_pos() {
                    if let Some(obj_id) = self.process_under(pos, &published.dynamic) {
                        let obj_name = self
                            .canvas
                            .objects
                            .iter()
                            .find(|o| o.id == obj_id)
                            .map(|o| o.name.clone())
                            .unwrap_or_default();
                        self.dialogs.edit_target = Some(obj_id);
                        self.dialogs.edit_input = obj_name;
                        self.selected_objects.clear();
                        self.drag_object = None;
                        self.drag_metric_source = None;
                        self.selection_box_start = None;
                    }
                }
            }

            // ---- Начало взаимодействия: драг отдела за шапку, драг показателя или selection box ----
            if canvas_response.drag_started_by(PointerButton::Primary)
                && self.drag_object.is_none()
                && self.drag_metric_source.is_none()
            {
                if let Some(pos) = canvas_response.interact_pointer_pos() {
                    if self.header_under(pos).is_some() {
                        if let Some(id) = self.header_under(pos) {
                            self.start_drag_object(id, pos);
                        }
                    } else if let Some((obj_id, dir, name)) = self.drag_metric_under(pos) {
                        // Начало перетаскивания показателя
                        if self.canvas.objects.iter().any(|o| o.id == obj_id && o.kind == NodeKind::Process) {
                            self.drag_metric_source = Some((obj_id, dir, name));
                            self.drag_metric_start = Some(pos);
                        }
                    } else {
                        // Процесс — начинаем его перетаскивание; иначе — выделение рамкой.
                        if let Some(obj_id) = self.process_under(pos, &published.dynamic) {
                            self.start_drag_process(obj_id, pos);
                        } else {
                            self.selection_box_start = Some(pos);
                        }
                    }
                }
            }

            // ---- Отрисовка selection box и обновление позиции драга при перетаскивании ----
            if canvas_response.dragged_by(PointerButton::Primary) {
                if self.drag_object.is_some() {
                    if let Some(cur) = canvas_response.interact_pointer_pos() {
                        let is_process_drag = self
                            .canvas
                            .objects
                            .iter()
                            .any(|o| o.id == self.drag_object.as_deref().unwrap_or("")
                                && o.kind == NodeKind::Process);
                        if is_process_drag {
                            let theme = self.canvas.theme.clone();
                            self.update_drag_process(ui, cur, &theme);
                        } else {
                            self.update_drag_object(cur);
                        }
                    }
                }
                if self.drag_metric_source.is_some() {
                    if let Some(cur) = canvas_response.interact_pointer_pos() {
                        if let Some((src_id, _, src_name)) = &self.drag_metric_source {
                            if let Some((_, _, _, src_rect)) = self.canvas.drag_metric_hotspots.iter()
                                .find(|(id, _, name, _)| id == src_id && name == src_name)
                            {
                                ui.painter().line_segment(
                                    [src_rect.center(), cur],
                                    egui::Stroke::new(2.0, self.canvas.theme.accent),
                                );
                            }
                        }
                    }
                }
                if self.selection_box_start.is_some() {
                    if let Some(cur) = canvas_response.interact_pointer_pos() {
                        let start = self.selection_box_start.unwrap();
                        let x = start.x.min(cur.x);
                        let y = start.y.min(cur.y);
                        let w = (cur.x - start.x).abs();
                        let h = (cur.y - start.y).abs();
                        if w > 4.0 && h > 4.0 {
                            let box_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h));
                            ui.painter().rect_stroke(
                                box_rect,
                                0.0,
                                egui::Stroke::new(2.0, self.canvas.theme.text),
                                egui::StrokeKind::Middle,
                            );
                        }
                    }
                }
            }

            // ---- Завершение перетаскивания объекта ----
            if self.drag_object.is_some()
                && !ui.input(|i| i.pointer.button_down(PointerButton::Primary))
            {
                let is_process_drag = self
                    .canvas
                    .objects
                    .iter()
                    .any(|o| o.id == self.drag_object.as_deref().unwrap_or("")
                        && o.kind == NodeKind::Process);
                if is_process_drag {
                    self.end_drag_process();
                } else {
                    self.end_drag_object();
                }
            }

            // ---- Завершение перетаскивания показателя ----
            if self.drag_metric_source.is_some()
                && !ui.input(|i| i.pointer.button_down(PointerButton::Primary))
            {
                let (src_id, src_dir, src_name) = self.drag_metric_source.take().unwrap();
                self.drag_metric_start = None;
                if let Some(cur) = canvas_response.interact_pointer_pos() {
                    // Реордер строки внутри того же контейнера (ЗАПРОС/РЕЗУЛЬТАТ одного объекта).
                    let hovered_same = self.canvas.drag_metric_hotspots.iter()
                        .find(|(oid, d, name, rect)| {
                            *oid == src_id && *d == src_dir && rect.contains(cur)
                        })
                        .map(|(_, _, name, _)| name.clone());
                    if let Some(hovered_name) = hovered_same {
                        if let Some(obj) = self.canvas.objects.iter().find(|o| o.id == src_id) {
                            let entries = match src_dir {
                                Direction::Request => &obj.requests,
                                Direction::Result => &obj.results,
                            };
                            if let Some(to_ix) = entries.iter().position(|e| e.name == hovered_name) {
                                // Найти from_ix для перетаскиваемой записи.
                                if let Some(from_ix) = entries.iter().position(|e| e.name == src_name) {
                                    if to_ix != from_ix {
                                        self.apply_cmd(polygram_core::Command::ReorderMetricEntry {
                                            object: src_id.clone(),
                                            dir: src_dir,
                                            from_ix,
                                            to_ix,
                                        });
                                    }
                                }
                                self.drag_metric_source = None;
                                return;
                            }
                        }
                    }
                    // Соединение показателя с объектом-целью: ищем любой объект под курсором.
                    if let Some(dst_id) = self.canvas.object_at_any(cur, &published.dynamic) {
                        if src_id != dst_id {
                            let geometry = self.engine.geometry();
                            let cmds = polygram_core::drag_domain::compute_metric_drop(
                                &polygram_core::drag_domain::MetricDropContext {
                                    geometry,
                                    source_object: src_id.clone(),
                                    source_dir: src_dir,
                                    entry_name: src_name.clone(),
                                    target_id: dst_id.to_owned(),
                                },
                            );
                                if !cmds.is_empty() {
                                    if let Err((_, e)) = self.engine.apply_batch(
                                        polygram_core::types::Scope::Root,
                                        cmds,
                                    ) {
                                        // Если показатель-def не существует, создаём его и повторяем.
                                        if matches!(
                                            e,
                                            polygram_core::error::EngineError::MetricDefNotFound(_)
                                        ) {
                                            let def_cmds = vec![
                                                polygram_core::Command::AddMetricDef {
                                                    id: Some(src_name.clone()),
                                                    name: src_name.clone(),
                                                    kind: polygram_core::types::MetricKind::Number,
                                                    unit: String::new(),
                                                    api: None,
                                                },
                                            ];
                                            let _ = self.engine.apply_batch(
                                                polygram_core::types::Scope::Root,
                                                def_cmds,
                                            );
                                            let geometry = self.engine.geometry();
                                            let cmds2 =
                                                polygram_core::drag_domain::compute_metric_drop(
                                                    &polygram_core::drag_domain::MetricDropContext {
                                                        geometry,
                                                        source_object: src_id.clone(),
                                                        source_dir: src_dir,
                                                        entry_name: src_name.clone(),
                                                        target_id: dst_id.to_owned(),
                                                    },
                                                );
                                            if !cmds2.is_empty() {
                                                let _ = self.engine.apply_batch(
                                                    polygram_core::types::Scope::Root,
                                                    cmds2,
                                                );
                                            }
                                        }
                                    }
                                self.sync_canvas_from_engine();
                            }
                        }
                    }
                }
            }

            // ---- Завершение выделения рамкой (selection box) ----
            if self.selection_box_start.is_some()
                && !ui.input(|i| i.pointer.button_down(PointerButton::Primary))
            {
                if let (Some(start), Some(end)) = (
                    self.selection_box_start,
                    canvas_response.interact_pointer_pos(),
                ) {
                    let x = start.x.min(end.x);
                    let y = start.y.min(end.y);
                    let w = (end.x - start.x).abs();
                    let h = (end.y - start.y).abs();
                    self.selection_box_start = None;
                    if w > 4.0 && h > 4.0 {
                        let cell_px = self.canvas.cell_px();
                        let box_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h));
                        let ctrl = ui.ctx().input(|i| i.modifiers.ctrl);
                        let mut newly_selected: Vec<String> = Vec::new();
                        for object in &self.canvas.objects {
                            let r = &object.rect;
                            let rect_px = if object.kind == NodeKind::Process {
                                let zoom = self.canvas.camera.zoom;
                                let card_w = 512.0 * zoom;
                                let card_h = polygram_canvas::paint::object::process_total_height(
                                    object,
                                    &published.dynamic,
                                    zoom,
                                    card_w,
                                );
                                Rect::from_min_size(
                                    self.canvas.cell_to_screen(r.x, r.y),
                                    egui::Vec2::new(card_w, card_h),
                                )
                            } else {
                                Rect::from_min_size(
                                    self.canvas.cell_to_screen(r.x, r.y),
                                    egui::Vec2::new(r.w as f32 * cell_px, r.h as f32 * cell_px),
                                )
                            };
                            if box_rect.intersects(rect_px) {
                                newly_selected.push(object.id.clone());
                            }
                        }
                        if ctrl {
                            for id in &newly_selected {
                                if !self.selected_objects.iter().any(|s| s == id) {
                                    self.selected_objects.push(id.clone());
                                }
                            }
                        } else {
                            self.selected_objects = newly_selected;
                        }
                    }
                }
            }
        });

        interaction::context_menu(self, ui.ctx(), &published, self.canvas.theme);
        interaction::rename_dialog(self, ui.ctx());
        interaction::rename::inline_rename_editor(self, ui.ctx(), &published);
        interaction::reparent_confirm::handle_reparent_confirm(self, ui.ctx(), &published);

        ui.ctx().request_repaint();
    }
}

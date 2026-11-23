//! Состояние приложения Polygram.
//! Хранит Canvas, движок ядра, отслеживает клики по холсту, выделение объектов,
//! перетаскивание отделов за шапку, диалог переименования и диалоги показателей.

use std::collections::HashSet;

use egui::{Pos2, PointerButton, Rect, Vec2};
use polygram_canvas::{Canvas, paint::constants::PROCESS_HEADER_H_PX};
use polygram_core::{Command, Engine, runtime::Published, types::{ContainerDirection, Direction, NodeKind}};
use polygram_core::drag_domain::{compute_metric_drop_object, compute_object_drop_commands, ObjectDropContext};
use polygram_core::geometry::{Geometry, ObjectGeom};
use polygram_core::layout::Rect as CoreRect;

mod dialogs;
mod icons;
mod departments;
mod interaction;

/// Найти свободную клетку внутри отдела для размещения объекта размером w×h клеток.
/// Поиск ограничивается внутренней областью отдела (под шапкой, между детьми).
fn find_free_cell_in_dept(
    objects: &[ObjectGeom],
    dept_id: &str,
    w: i32,
    h: i32,
    exclude_id: &str,
) -> Option<(i32, i32)> {
    let dept = objects.iter().find(|o| o.id == dept_id && o.kind == NodeKind::Department)?;
    // Внутренняя область отдела: отступ 1 клетка по бокам, шапка занимает верхнюю строку.
    let inner_x0 = dept.rect.x + 1;
    let inner_y0 = dept.rect.y + 2;
    let inner_x1 = dept.rect.x + dept.rect.w - 1;
    let inner_y1 = dept.rect.y + dept.rect.h - 1;
    if inner_x1 < inner_x0 + w || inner_y1 < inner_y0 + h {
        return None;
    }
    // Сначала пробуем клетки рядом с верхним левым углу внутренней области, по строкам.
    for y in inner_y0..=inner_y1.saturating_sub(h) {
        for x in inner_x0..=inner_x1.saturating_sub(w) {
            let r = CoreRect::new(x, y, w, h);
            let free = objects.iter().all(|o| {
                o.id == exclude_id || o.id == dept_id || !o.rect.overlaps(&r)
            });
            if free {
                return Some((x, y));
            }
        }
    }
    None
}

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
    /// Объекты, которые сейчас перетаскиваются (один или группа выделенных).
    drag_objects: Vec<String>,
    /// Смещение курсора относительно левого верхнего угла каждого объекта в клетках.
    drag_offsets: Vec<(f32, f32)>,
    /// Начальная позиция мыши при начале драга (экран).
    drag_start_screen: Option<Pos2>,
    /// Стартовые клетки перетаскиваемых объектов (для отмены при возврате на место).
    drag_start_cells: Vec<(i32, i32)>,
    /// Отдел, над которым нависает перетаскиваемый объект.
    drag_over: Option<String>,
    /// Свободная клетка для призрачной копии при перетаскивании объекта в отдел.
    drag_ghost: Option<(i32, i32)>,
    /// Направление контейнера перетаскиваемого объекта (для Контейнера/Показателя).
    drag_object_dir: Option<ContainerDirection>,
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
            drag_objects: Vec::new(),
            drag_offsets: Vec::new(),
            drag_start_screen: None,
            drag_start_cells: Vec::new(),
            drag_over: None,
            drag_ghost: None,
            drag_object_dir: None,
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
        let mut geometry = self.engine.geometry();
        // Отдел не имеет фиксированного размера: рамка обтягивается под детей.
        crate::app::departments::fit_departments_to_children(&mut geometry, None);
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
        let zoom = self.canvas.camera.zoom;
        // Заголовки отделов.
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
        // Заголовки процессов (только верхняя часть карточки).
        for object in &self.canvas.objects {
            if object.kind != NodeKind::Process {
                continue;
            }
            let r = &object.rect;
            let rect_px = self.canvas.cell_to_screen(r.x, r.y);
            let header_h = PROCESS_HEADER_H_PX * zoom;
            let header_rect =
                egui::Rect::from_min_size(rect_px, egui::Vec2::new(512.0 * zoom, header_h));
            if header_rect.contains(screen_pos) {
                return Some(object.id.clone());
            }
        }
        None
    }

    /// Найти заголовок контейнера под заданной экранной позицией.
    /// Включает контейнерные шапки внутри карточек процессов
    /// (у таких записей id = id процесса, direction = направление контейнера).
    fn container_header_under(&self, screen_pos: Pos2) -> Option<(String, ContainerDirection)> {
        for (id, dir, rect) in &self.canvas.container_header_hotspots {
            if rect.contains(screen_pos) {
                let container_dir = match dir {
                    polygram_core::types::Direction::Request => ContainerDirection::Request,
                    polygram_core::types::Direction::Result => ContainerDirection::Result,
                };
                return Some((id.clone(), container_dir));
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

    /// Начать перетаскивание одного объекта или группы выделенных объектов.
    /// Если кликнутый объект уже в выделении — тащит всю группу (как в Figma),
    /// иначе — только этот объект.
    fn start_drag_objects(&mut self, ids: Vec<String>, screen_pos: Pos2) {
        self.drag_objects = ids.clone();
        self.drag_offsets = Vec::new();
        self.drag_start_cells = Vec::new();
        for id in &ids {
            if let Some(obj) = self.canvas.objects.iter().find(|o| o.id == *id) {
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
                self.drag_offsets.push((
                    click_cell_x - top_left_cell_x,
                    click_cell_y - top_left_cell_y,
                ));
                self.drag_start_cells.push((obj.rect.x, obj.rect.y));
            }
        }
        self.drag_start_screen = Some(screen_pos);
        self.drag_over = None;
        self.drag_ghost = None;
        self.drag_object_dir = None;
    }

    /// Очистить состояние драга без сброса позиций объектов.
    /// Используется после успешного apply_batch, когда позиции уже применены.
    fn clear_drag_state(&mut self) {
        self.drag_objects = Vec::new();
        self.drag_offsets = Vec::new();
        self.drag_start_cells = Vec::new();
        self.drag_start_screen = None;
        self.drag_over = None;
        self.drag_ghost = None;
        self.drag_object_dir = None;
    }

    /// Сбросить состояние драга и вернуть объекты на исходные позиции
    /// (используется при отмене драга — открытии диалога или возврате на место).
    fn cancel_drag(&mut self) {
        // Вернуть объекты на исходные позиции, чтобы они не «залипли» на экране.
        for (i, id) in self.drag_objects.iter().enumerate() {
            let start_cell = self.drag_start_cells.get(i).copied().unwrap_or((0, 0));
            if let Some(o) = self.canvas.objects.iter_mut().find(|o| o.id == *id) {
                o.rect.x = start_cell.0;
                o.rect.y = start_cell.1;
            }
        }
        self.clear_drag_state();
    }

    /// Применить текущую позицию курсора ко всем перетаскиваемым объектам
    /// (привязка к клеткам). Отдел тащит своих детей, не входящих в группу.
    /// Также подсвечивает отдел под курсором.
    fn update_drag_objects(&mut self, ui: &mut egui::Ui, cursor: Pos2) {
        if self.drag_objects.is_empty() {
            return;
        }
        let cursor_world_x = (cursor.x / self.canvas.camera.zoom - self.canvas.camera.offset.x)
            / self.canvas.cell_size;
        let cursor_world_y = (cursor.y / self.canvas.camera.zoom - self.canvas.camera.offset.y)
            / self.canvas.cell_size;
        let drag_set: HashSet<&str> =
            self.drag_objects.iter().map(|s| s.as_str()).collect();
        let mut over_dept: Option<String> = None;
        // Определить отдел под курсором (исключая перетаскиваемые).
        if let Some(under_id) = self.canvas.objects.iter().find(|o| {
            let cx = (cursor.x / self.canvas.camera.zoom - self.canvas.camera.offset.x)
                / self.canvas.cell_size;
            let cy = (cursor.y / self.canvas.camera.zoom - self.canvas.camera.offset.y)
                / self.canvas.cell_size;
            let cursor_cell = (cx.round() as i32, cy.round() as i32);
            o.id != self.drag_objects.first().map(|s| s.as_str()).unwrap_or("")
                && !drag_set.contains(o.id.as_str())
                && cursor_cell.0 >= o.rect.x && cursor_cell.0 < o.rect.x + o.rect.w
                && cursor_cell.1 >= o.rect.y && cursor_cell.1 < o.rect.y + o.rect.h
        }).map(|o| o.id.clone()) {
            if let Some(under_obj) = self.canvas.objects.iter().find(|o| o.id == under_id) {
                match under_obj.kind {
                    NodeKind::Department => {
                        over_dept = Some(under_id);
                    }
                    NodeKind::Process => {
                        if let Some(dept_id) = &under_obj.parent {
                            if self.canvas.objects.iter().any(|o| o.id == *dept_id && o.kind == NodeKind::Department)
                            {
                                over_dept = Some(dept_id.clone());
                            } else {
                                over_dept = Some(under_id);
                            }
                        } else {
                            over_dept = Some(under_id);
                        }
                    }
                    _ => {}
                }
            }
        }
        self.drag_over = over_dept.clone();

        // Переместить каждый объект группы, сохраняя относительные позиции.
        for (i, id) in self.drag_objects.iter().enumerate() {
            let target_x = (cursor_world_x - self.drag_offsets[i].0).round() as i32;
            let target_y = (cursor_world_y - self.drag_offsets[i].1).round() as i32;
            if let Some(dragged) = self.canvas.objects.iter_mut().find(|o| o.id == *id) {
                let dx = target_x - dragged.rect.x;
                let dy = target_y - dragged.rect.y;
                dragged.rect.x = target_x;
                dragged.rect.y = target_y;
                // Отдел тащит всех своих прямых детей, кроме тех, что в группе перетаскивания.
                if dragged.kind == NodeKind::Department {
                    for child in self.canvas.objects.iter_mut().filter(|o| {
                        o.parent.as_deref() == Some(id) && !drag_set.contains(o.id.as_str())
                    }) {
                        child.rect.x += dx;
                        child.rect.y += dy;
                    }
                }
            }
        }

        let painter = ui.painter();
        let cell_px = self.canvas.cell_px();
        // Подсветить отдел, над которым нависает группа.
        if let Some(dept_id) = &over_dept {
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
                    egui::Stroke::new(2.0 * self.canvas.camera.zoom, self.canvas.theme.accent),
                    egui::StrokeKind::Middle,
                );
            }
        }
    }

    /// Завершить перетаскивание группы объектов: применить MoveObject для каждого,
    /// чья позиция изменилась. Отдел перевозит детей, не входящих в группу.
    /// Для одного Процесса, брошенного в другой отдел — открыть диалог подтверждения
    /// переноса (сохранение старого поведения).
    fn end_drag_objects(&mut self) {
        if self.drag_objects.is_empty() {
            self.drag_start_screen = None;
            return;
        }
        let drag_set: HashSet<&str> =
            self.drag_objects.iter().map(|s| s.as_str()).collect();

        // Один Процесс над отделом — диалог подтверждения переноса (как раньше).
        if self.drag_objects.len() == 1 {
            let id = self.drag_objects[0].clone();
            if let Some(obj) = self.canvas.objects.iter().find(|o| o.id == id) {
                if obj.kind == NodeKind::Process {
                    if let Some(dept_id) = &self.drag_over {
                        if self.canvas.objects.iter().any(|o| o.id == *dept_id && o.kind == NodeKind::Department) {
                            let start_cell = self.drag_start_cells.get(0).copied().unwrap_or((0, 0));
                            let object_size = (obj.rect.w, obj.rect.h);
                            let drop_cell = find_free_cell_in_dept(
                                &self.canvas.objects,
                                dept_id,
                                object_size.0,
                                object_size.1,
                                &id,
                            )
                            .unwrap_or((obj.rect.x, obj.rect.y));
                            let already_in_dept = self.canvas.objects.iter().any(|o| {
                                o.id == id && o.parent.as_deref() == Some(dept_id)
                            });
                            if already_in_dept {
                                if drop_cell != start_cell {
                                    self.apply_batch(vec![Command::MoveObject {
                                        object: id.clone(),
                                        at: [drop_cell.0, drop_cell.1],
                                    }]);
                                } else {
                                    if let Some(o) = self.canvas.objects.iter_mut().find(|o| o.id == id) {
                                        o.rect.x = start_cell.0;
                                        o.rect.y = start_cell.1;
                                    }
                                }
                                self.clear_drag_state();
                                return;
                            }
                            self.dialogs.reparent_confirm = Some((
                                id.clone(),
                                dept_id.clone(),
                                drop_cell,
                                start_cell,
                                NodeKind::Process,
                            ));
                            self.cancel_drag();
                            return;
                        }
                    }
                }
            }
        }

        // Свободное место: переместить все объекты группы, сохраняя относительные позиции.
        let mut cmds: Vec<Command> = Vec::new();
        let mut any_moved = false;
        for (i, id) in self.drag_objects.iter().enumerate() {
            let start_cell = self.drag_start_cells.get(i).copied().unwrap_or((0, 0));
            let Some(obj) = self.canvas.objects.iter().find(|o| o.id == *id).cloned() else {
                continue;
            };
            if (obj.rect.x, obj.rect.y) == start_cell {
                continue;
            }
            any_moved = true;
            cmds.push(Command::MoveObject {
                object: id.clone(),
                at: [obj.rect.x, obj.rect.y],
            });
            // Отдел перевозит детей, не входящих в группу.
            if obj.kind == NodeKind::Department {
                for child in self.canvas.objects.iter().filter(|o| {
                    o.parent.as_deref() == Some(id) && !drag_set.contains(o.id.as_str())
                }) {
                    cmds.push(Command::MoveObject {
                        object: child.id.clone(),
                        at: [child.rect.x, child.rect.y],
                    });
                }
            }
        }
        if !any_moved {
            // Вернуть на исходные позиции (отмена).
            for (i, id) in self.drag_objects.iter().enumerate() {
                let start_cell = self.drag_start_cells.get(i).copied().unwrap_or((0, 0));
                if let Some(o) = self.canvas.objects.iter_mut().find(|o| o.id == *id) {
                    o.rect.x = start_cell.0;
                    o.rect.y = start_cell.1;
                }
            }
        } else {
            self.apply_batch(cmds);
        }
        self.clear_drag_state();
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

    /// Начать перетаскивание контейнера как объекта (единичная операция).
    fn start_drag_container(&mut self, id: String, dir: ContainerDirection, screen_pos: Pos2) {
        self.start_drag_objects(vec![id.clone()], screen_pos);
        self.drag_object_dir = Some(dir);
        self.drag_over = None;
        self.drag_ghost = None;
    }

    /// Начать перетаскивание показателя как объекта (единичная операция).
    fn start_drag_metric_object(&mut self, id: String, dir: ContainerDirection, screen_pos: Pos2) {
        self.start_drag_objects(vec![id.clone()], screen_pos);
        self.drag_object_dir = Some(dir);
        self.drag_over = None;
        self.drag_ghost = None;
    }

    /// Вычислить отдел под курсором для перетаскиваемого объекта (контейнер/показатель).
    fn object_drag_preview(&self, cursor: Pos2) -> (Option<String>, Option<(i32, i32)>) {
        let Some(drag_id) = self.drag_objects.first().map(|s| s.as_str()) else {
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

        // Найти объект под курсором (исключая перетаскиваемый), определить отдел для подсветки.
        let mut over: Option<String> = None;
        if let Some(under_id) = self.canvas.objects.iter().find(|o| {
            o.id != drag_id
                && cursor_cell.0 >= o.rect.x && cursor_cell.0 < o.rect.x + o.rect.w
                && cursor_cell.1 >= o.rect.y && cursor_cell.1 < o.rect.y + o.rect.h
        }).map(|o| o.id.clone()) {
            if let Some(under_obj) = self.canvas.objects.iter().find(|o| o.id == under_id) {
                match under_obj.kind {
                    NodeKind::Department => {
                        over = Some(under_id);
                    }
                    NodeKind::Process => {
                        if let Some(dept_id) = &under_obj.parent {
                            if self.canvas.objects.iter().any(|o| o.id == *dept_id && o.kind == NodeKind::Department)
                            {
                                over = Some(dept_id.clone());
                            } else {
                                over = Some(under_id);
                            }
                        } else {
                            over = Some(under_id);
                        }
                    }
                    NodeKind::Container | NodeKind::Metric => {}
                    _ => {}
                }
            }
        }

        let ghost = if over.is_some() {
            // Над отделом — ищем свободную клетку внутри отдела для призрака.
            let dept_id = over.as_deref().unwrap();
            if let Some(_dept) = self.canvas.objects.iter().find(|o| o.id == dept_id && o.kind == NodeKind::Department) {
                find_free_cell_in_dept(
                    &self.canvas.objects,
                    dept_id,
                    dragged_obj.rect.w,
                    dragged_obj.rect.h,
                    drag_id,
                )
            } else {
                None
            }
        } else {
            // Свободное место — позиция курсора как призрак.
            Some(cursor_cell)
        };

        (over, ghost)
    }
    /// Завершить перетаскивание контейнера или показателя как объекта.
    /// Вычисляет команды через доменную логику и применяет их.
    fn end_drag_container_or_metric(&mut self) {
        let Some(drag_id) = self.drag_objects.first().cloned() else {
            self.drag_object_dir = None;
            return;
        };
        let start_cell = self.drag_start_cells.get(0).copied().unwrap_or((0, 0));
        let Some(obj) = self.canvas.objects.iter().find(|o| o.id == drag_id).cloned() else {
            self.drag_objects = Vec::new();
            self.drag_offsets = Vec::new();
            self.drag_start_cells = Vec::new();
            self.drag_start_screen = None;
            self.drag_over = None;
            self.drag_ghost = None;
            self.drag_object_dir = None;
            return;
        };

        // Конечная клетка.
        let drop_cell = if self.drag_over.as_ref().map_or(false, |dept_id| {
            self.canvas.objects.iter().any(|o| o.id == *dept_id && o.kind == NodeKind::Department)
        }) {
            let object_size = (obj.rect.w, obj.rect.h);
            find_free_cell_in_dept(
                &self.canvas.objects,
                self.drag_over.as_deref().unwrap(),
                object_size.0,
                object_size.1,
                &drag_id,
            )
            .unwrap_or((obj.rect.x, obj.rect.y))
        } else {
            self.drag_ghost.unwrap_or((obj.rect.x, obj.rect.y))
        };

        // Если позиция не изменилась — отмена.
        if drop_cell == start_cell {
            self.drag_objects = Vec::new();
            self.drag_offsets = Vec::new();
            self.drag_start_cells = Vec::new();
            self.drag_start_screen = None;
            self.drag_over = None;
            self.drag_ghost = None;
            self.drag_object_dir = None;
            return;
        }

        let geometry = Geometry {
            rev: 0,
            objects: self.canvas.objects.clone(),
            edges: Vec::new(),
        };
        let ctx = ObjectDropContext {
            geometry,
            dragged_id: drag_id.clone(),
            start_cell,
            drop_cell,
            over_dept: self.drag_over.clone(),
            dragged_size: (obj.rect.w, obj.rect.h),
            dragged_dir: self.drag_object_dir,
        };
        let result = compute_object_drop_commands(&ctx);

        if !result.commands.is_empty() {
            self.apply_batch(result.commands);
        }

        // Для перетаскивания контейнера/показателя как объекта НЕ перемещаем
        // исходный объект (доменная логика создаёт новый Процесс и копирует показатели).
        // Перемещаем только при обычном драге объекта без dragged_dir.
        if self.drag_object_dir.is_none() {
            if let Some(o) = self.canvas.objects.iter_mut().find(|o| o.id == drag_id) {
                o.rect.x = result.final_cell.0;
                o.rect.y = result.final_cell.1;
            }
        }

        self.drag_objects = Vec::new();
        self.drag_offsets = Vec::new();
        self.drag_start_cells = Vec::new();
        self.drag_start_screen = None;
        self.drag_over = None;
        self.drag_ghost = None;
        self.drag_object_dir = None;
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

            // Отделы растягиваются под содержимое на каждом кадре, включая
            // позицию перетаскиваемого объекта (плавно «на лету»).
            let predicted = self.geometry_with_predictions();
            self.canvas.objects = predicted.objects;

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
                    } else if let Some((obj_id, dir, _rect)) = self.canvas.add_metric_hotspots.iter()
                        .find(|(_, _, rect)| rect.contains(pos))
                    {
                        // Клик на "+ Добавить показатель" — открыть диалог добавления показателя.
                        let n = match dir {
                            Direction::Request => self.canvas.objects.iter()
                                .find(|o| o.id == *obj_id)
                                .map(|o| o.requests.len())
                                .unwrap_or(0),
                            Direction::Result => self.canvas.objects.iter()
                                .find(|o| o.id == *obj_id)
                                .map(|o| o.results.len())
                                .unwrap_or(0),
                        };
                        self.dialogs.add_metric_defaults(obj_id.clone(), *dir, n);
                        // Сбрасываем любое состояние драга, чтобы клик на '+'
                        // не затащил объект за собой.
                        self.cancel_drag();
                    } else {
                        // Проверяем клик по процессу.
                        let mut clicked_process = false;
                        if let Some(obj_id) = self.process_under(pos, &published.dynamic) {
                            self.selection_box_start = None;
                            self.toggle_select(obj_id.clone(), ctrl);
                            clicked_process = true;
                        }
                        if !clicked_process {
                            // Клик по пустому месту — сбрасываем выделение.
                            self.selected_objects.clear();
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
                        self.cancel_drag();
                        self.drag_metric_source = None;
                        self.selection_box_start = None;
                    }
                }
            }

            // ---- Начало взаимодействия: драг отдела/процесса за шапку, драг контейнера, драг показателя или selection box ----
            if canvas_response.drag_started_by(PointerButton::Primary)
                && self.drag_objects.is_empty()
                && self.drag_metric_source.is_none()
                && self.dialogs.add_metric_target.is_none()
            {
                if let Some(pos) = canvas_response.interact_pointer_pos() {
                    // Не начинаем драг, если клик на кнопке "+ Добавить показатель":
                    let on_add_metric = self.canvas.add_metric_hotspots.iter()
                        .any(|(_, _, rect)| rect.contains(pos));
                    if on_add_metric {
                        // Пусть клик-обработчик ниже откроет диалог добавления показателя.
                    } else if let Some((id, dir)) = self.container_header_under(pos) {
                        // Шапка контейнера (включая шапку внутри карточки процесса) — драг контейнера.
                        self.start_drag_container(id, dir, pos);
                    } else if let Some(id) = self.header_under(pos) {
                        // Заголовок отдела или процесса — драг только из шапки.
                        // Если объект уже выделен — тащим всю группу (как в Figma), иначе только его.
                        let ctrl = ui.ctx().input(|i| i.modifiers.ctrl);
                        let already_selected = self.selected_objects.contains(&id);
                        if !ctrl && !already_selected {
                            self.selected_objects = vec![id.clone()];
                        }
                        self.start_drag_objects(self.selected_objects.clone(), pos);
                    } else if let Some((obj_id, dir, name)) = self.drag_metric_under(pos) {
                        // Начало перетаскивания показателя (соединение/переордер записей).
                        if self.canvas.objects.iter().any(|o| o.id == obj_id && o.kind == NodeKind::Process) {
                            self.drag_metric_source = Some((obj_id, dir, name));
                            self.drag_metric_start = Some(pos);
                        }
                    } else {
                        self.selection_box_start = Some(pos);
                    }
                }
            }

            // ---- Отрисовка selection box и обновление позиции драга при перетаскивании ----
            if canvas_response.dragged_by(PointerButton::Primary) {
                if !self.drag_objects.is_empty() {
                    if let Some(cur) = canvas_response.interact_pointer_pos() {
                        if self.drag_object_dir.is_some() {
                            // Драг контейнера/показателя как объекта (включая шапку контейнера внутри карточки процесса).
                            // Исходный объект НЕ перемещается — доменная логика создаёт новые объекты.
                            // Вычисляем только отделы под курсором и призрака.
                            let (over, ghost) = self.object_drag_preview(cur);
                            self.drag_over = over.clone();
                            self.drag_ghost = ghost;
                        } else {
                            self.update_drag_objects(ui, cur);
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
            if !self.drag_objects.is_empty()
                && !ui.input(|i| i.pointer.button_down(PointerButton::Primary))
            {
                // Если открыт диалог — отменяем драг без перемещения объекта.
                if self.dialogs.add_metric_target.is_some() || self.right_click_object.is_some() {
                    self.cancel_drag();
                } else {
                    if self.drag_object_dir.is_some() {
                        // Драг контейнера/показателя как объекта (включая шапку контейнера внутри карточки процесса).
                        self.end_drag_container_or_metric();
                    } else {
                        self.end_drag_objects();
                    }
                }
            }

            // ---- Завершение перетаскивания показателя ----
            if self.drag_metric_source.is_some()
                && !ui.input(|i| i.pointer.button_down(PointerButton::Primary))
            {
                let (src_id, src_dir, src_name) = self.drag_metric_source.take().unwrap();
                self.drag_metric_start = None;
                if let Some(cur) = canvas_response.interact_pointer_pos() {
                    let zoom = self.canvas.camera.zoom;
                    let offset = self.canvas.camera.offset;
                    let cell_size = self.canvas.cell_size;
                    let cur_cell = (
                        ((cur.x / zoom - offset.x) / cell_size).round() as i32,
                        ((cur.y / zoom - offset.y) / cell_size).round() as i32,
                    );
                    let under_any = self.canvas.objects.iter().any(|o| {
                        o.rect.x <= cur_cell.0
                            && cur_cell.0 < o.rect.x + o.rect.w
                            && o.rect.y <= cur_cell.1
                            && cur_cell.1 < o.rect.y + o.rect.h
                    });
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
                            }
                        }
                        self.sync_canvas_from_engine();
                    } else if !under_any {
                        // Свободное место: создать Процесс и разместить показатель
                        // в его контейнере с противоположным направлением (как при дропе Контейнера).
                        // Определение показателя должно существовать, иначе AddMetricEntry не применится.
                        if !self.engine.metric_def_views().iter().any(|m| m.name == src_name || m.id == src_name) {
                            let _ = self.engine.apply_batch(
                                polygram_core::types::Scope::Root,
                                vec![polygram_core::Command::AddMetricDef {
                                    id: Some(src_name.clone()),
                                    name: src_name.clone(),
                                    kind: polygram_core::types::MetricKind::Number,
                                    unit: String::new(),
                                    api: None,
                                }],
                            );
                        }
                        let geometry = self.engine.geometry();
                        let dragged_dir = match src_dir {
                            Direction::Request => ContainerDirection::Request,
                            Direction::Result => ContainerDirection::Result,
                        };
                        let cmds = polygram_core::drag_domain::compute_metric_drop_object(
                            &geometry,
                            &src_name,
                            dragged_dir,
                            cur_cell,
                            cur_cell,
                            None,
                            (2, 2),
                            None,
                        );
                        if !cmds.commands.is_empty() {
                            let _ = self.engine.apply_batch(
                                polygram_core::types::Scope::Root,
                                cmds.commands,
                            );
                        }
                        self.sync_canvas_from_engine();
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
        interaction::add_metric::handle_add_metric_dialog(self, ui.ctx(), &published);
        interaction::rename_dialog(self, ui.ctx());
        interaction::rename::inline_rename_editor(self, ui.ctx(), &published);
        interaction::reparent_confirm::handle_reparent_confirm(self, ui.ctx(), &published);

        ui.ctx().request_repaint();
    }
}

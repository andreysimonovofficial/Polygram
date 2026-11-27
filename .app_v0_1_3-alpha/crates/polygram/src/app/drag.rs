//! Локальное состояние и жизненный цикл перетаскивания объектов.
//! Отвечает за:
//! - начало/конец/отмену драга объектов и группы выделенных;
//! - плавное визуальное перемещение во время драга (rect привязан к сетке,
//!   визуально сдвигается на фракцию клетки);
//! - привязку к сетке при сбросе (Drop);
//! - драг контейнера/показателя как объекта.

use std::collections::HashSet;

use egui::{Pos2, PointerButton, Rect, Vec2};
use polygram_core::{
    Command,
    drag_domain::{compute_object_drop_commands, ObjectDropContext},
    geometry::{Geometry, ObjectGeom},
    layout::Rect as CoreRect,
    types::{ContainerDirection, Direction, NodeKind},
};
use crate::app::PolygramApp;
use super::interaction::place_object::PlaceObjectConfirmData;

/// Найти свободную клетку внутри отдела для размещения объекта размером w×h клеток.
/// Поиск ограничивается внутренней областью отдела (под шапкой, между детьми).
pub(crate) fn find_free_cell_in_dept(
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

impl PolygramApp {
    /// Начать перетаскивание одного объекта или группы выделенных объектов.
    /// Если кликнутый объект уже в выделении — тащит всю группу (как в Figma),
    /// иначе — только этот объект.
    pub(crate) fn start_drag_objects(&mut self, ids: Vec<String>, screen_pos: Pos2) {
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
        self.drag_visual_offset = None;
        self.drag_visual_ids = ids.iter().cloned().collect();
    }

    /// Очистить состояние драга без сброса позиций объектов.
    /// Используется после успешного apply_batch, когда позиции уже применены.
    pub(crate) fn clear_drag_state(&mut self) {
        self.drag_objects = Vec::new();
        self.drag_offsets = Vec::new();
        self.drag_start_cells = Vec::new();
        self.drag_start_screen = None;
        self.drag_over = None;
        self.drag_ghost = None;
        self.drag_object_dir = None;
        self.drag_visual_offset = None;
        self.drag_visual_ids.clear();
    }

    /// Сбросить состояние драга и вернуть объекты на исходные позиции
    /// (используется при отмене драга — открытии диалога или возврате на место).
    pub(crate) fn cancel_drag(&mut self) {
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
    pub(crate) fn update_drag_objects(&mut self, ui: &mut egui::Ui, cursor: Pos2) {
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
        // rect остаётся привязанным к сетке (целые клетки) для логики и сброса;
        // визуальное плавное смещение задаётся drag_visual_offset (фракция клетки)
        // и применяется при отрисовке.
        let mut visual_offset: Option<(f32, f32)> = None;
        let mut visual_ids: HashSet<String> = self.drag_objects.iter().cloned().collect();
        for (i, id) in self.drag_objects.iter().enumerate() {
            let smooth_x = cursor_world_x - self.drag_offsets[i].0;
            let smooth_y = cursor_world_y - self.drag_offsets[i].1;
            let target_x = smooth_x.round() as i32;
            let target_y = smooth_y.round() as i32;
            if visual_offset.is_none() {
                visual_offset = Some((smooth_x - target_x as f32, smooth_y - target_y as f32));
            }
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
                        visual_ids.insert(child.id.clone());
                    }
                }
            }
        }
        self.drag_visual_offset = visual_offset;
        self.drag_visual_ids = visual_ids;

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
                _ = rect_px;
            }
        }
    }

    /// Завершить перетаскивание группы объектов: применить MoveObject для каждого,
    /// чья позиция изменилась. Отдел перевозит детей, не входящих в группу.
    /// Для одного Процесса, брошенного в другой отдел — открыть диалог подтверждения
    /// переноса (сохранение старого поведения).
    pub(crate) fn end_drag_objects(&mut self) {
        if self.drag_objects.is_empty() {
            self.drag_start_screen = None;
            return;
        }
        let drag_set: HashSet<&str> =
            self.drag_objects.iter().map(|s| s.as_str()).collect();

        // Один объект (Процесс или Отдел) над Отделом — диалог «Поместить Объект в Объект».
        // Процесс в Процесс и Отдел в Процесс запрещены.
        if self.drag_objects.len() == 1 {
            let id = self.drag_objects[0].clone();
            if let Some(obj) = self.canvas.objects.iter().find(|o| o.id == id) {
                if obj.kind == NodeKind::Process || obj.kind == NodeKind::Department {
                    if let Some(over_id) = &self.drag_over {
                        if let Some(over_obj) = self.canvas.objects.iter().find(|o| o.id == *over_id)
                        {
                            match over_obj.kind {
                                NodeKind::Department => {
                                    let start_cell =
                                        self.drag_start_cells.get(0).copied().unwrap_or((0, 0));
                                    let object_size = (obj.rect.w, obj.rect.h);
                                    let already_in_dept = self.canvas.objects.iter().any(|o| {
                                        o.id == id && o.parent.as_deref() == Some(over_id)
                                    });
                                    let drop_cell = if already_in_dept {
                                        // Процесс уже находится в отделе: перемещаем на фактическую
                                        // позицию курсора (привязанную к сетке в update_drag_objects),
                                        // а не на первую свободную клетку отдела.
                                        (obj.rect.x, obj.rect.y)
                                    } else {
                                        find_free_cell_in_dept(
                                            &self.canvas.objects,
                                            over_id,
                                            object_size.0,
                                            object_size.1,
                                            &id,
                                        )
                                        .unwrap_or((obj.rect.x, obj.rect.y))
                                    };
                                    if already_in_dept {
                                        if drop_cell != start_cell {
                                            self.apply_batch(vec![Command::MoveObject {
                                                object: id.clone(),
                                                at: [drop_cell.0, drop_cell.1],
                                            }]);
                                        } else {
                                            if let Some(o) = self
                                                .canvas
                                                .objects
                                                .iter_mut()
                                                .find(|o| o.id == id)
                                            {
                                                o.rect.x = start_cell.0;
                                                o.rect.y = start_cell.1;
                                            }
                                        }
                                        self.clear_drag_state();
                                        return;
                                    }
                                    let object_name = obj.name.clone();
                                    let target_name = over_obj.name.clone();
                                    self.dialogs.place_object_confirm =
                                        Some(PlaceObjectConfirmData {
                                            object_id: id.clone(),
                                            object_name,
                                            object_kind: obj.kind,
                                            target_id: over_id.clone(),
                                            target_name,
                                            target_kind: over_obj.kind,
                                            drop_cell,
                                            start_cell,
                                        });
                                    self.cancel_drag();
                                    return;
                                }
                                NodeKind::Process => {
                                    // Запрещено: Процесс в Процесс или Отдел в Процесс.
                                    self.cancel_drag();
                                    return;
                                }
                                _ => {}
                            }
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
    pub(crate) fn drag_metric_under(&self, pos: Pos2) -> Option<(String, Direction, String)> {
        for (id, dir, name, rect) in &self.canvas.drag_metric_hotspots {
            if rect.contains(pos) {
                return Some((id.clone(), *dir, name.clone()));
            }
        }
        None
    }

    /// Начать перетаскивание контейнера как объекта (единичная операция).
    pub(crate) fn start_drag_container(&mut self, id: String, dir: ContainerDirection, screen_pos: Pos2) {
        self.start_drag_objects(vec![id.clone()], screen_pos);
        self.drag_object_dir = Some(dir);
        self.drag_over = None;
        self.drag_ghost = None;
    }

    /// Начать перетаскивание показателя как объекта (единичная операция).
    pub(crate) fn start_drag_metric_object(&mut self, id: String, dir: ContainerDirection, screen_pos: Pos2) {
        self.start_drag_objects(vec![id.clone()], screen_pos);
        self.drag_object_dir = Some(dir);
        self.drag_over = None;
        self.drag_ghost = None;
    }

    /// Вычислить отдел под курсором для перетаскиваемого объекта (контейнер/показатель).
    pub(crate) fn object_drag_preview(&self, cursor: Pos2) -> (Option<String>, Option<(i32, i32)>) {
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
    pub(crate) fn end_drag_container_or_metric(&mut self) {
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

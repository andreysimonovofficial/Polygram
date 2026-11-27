//! Перетаскивание контейнера и показателя как объекта.
//! Отвечает за:
//! - start_drag_container — начало драга контейнера (шапка контейнера в карточке процесса);
//! - start_drag_metric_object — начало драга показателя как объекта;
//! - object_drag_preview — отделы под курсором и призрачная копия;
//! - end_drag_container_or_metric — завершение драга контейнера/показателя через доменную логику.

use egui::{Pos2};
use polygram_core::{
    drag_domain::{compute_object_drop_commands, ObjectDropContext},
    geometry::Geometry,
    types::{ContainerDirection, NodeKind},
};
use crate::app::PolygramApp;
use super::free_cell::find_free_cell_in_dept;

impl PolygramApp {
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
                    self,
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
                self,
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

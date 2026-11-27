//! Обработка перетаскивания показателя: соединение показателей, реордер строк.

use egui::{Pos2, PointerButton};
use polygram_core::{
    drag_domain::compute_metric_drop_object,
    types::{ContainerDirection, Direction},
};
use polygram_core::runtime::Published;
use crate::app::PolygramApp;

impl PolygramApp {
    /// Начать перетаскивание показателя (соединение/переордер записей).
    pub(crate) fn start_drag_metric(
        &mut self,
        obj_id: String,
        dir: Direction,
        name: String,
        pos: Pos2,
    ) {
        if self.canvas.objects.iter().any(|o| o.id == obj_id && o.kind == polygram_core::types::NodeKind::Process) {
            self.drag_metric_source = Some((obj_id, dir, name));
            self.drag_metric_start = Some(pos);
        }
    }

    /// Отрисовать линию перетаскивания показателя и обработать его завершение.
    /// Возвращает `true`, если перетаскивание показателя завершено и состояние можно сбросить.
    pub(crate) fn handle_metric_drag(
        &mut self,
        ui: &egui::Ui,
        published: &Published,
    ) -> bool {
        if self.drag_metric_source.is_none() {
            return false;
        }

        if ui.input(|i| i.pointer.button_down(PointerButton::Primary)) {
            // Ещё тащим — рисуем линию
            if let Some(cur) = ui.input(|i| i.pointer.hover_pos()) {
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
            return false;
        }

        // Завершение перетаскивания показателя
        let (src_id, src_dir, src_name) = self.drag_metric_source.take().unwrap();
        self.drag_metric_start = None;
        if let Some(cur) = ui.input(|i| i.pointer.hover_pos()) {
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
                    }
                    self.drag_metric_source = None;
                    return true;
                }
            }

            // Соединение показателя с объектом-целью: определяем цель под курсором.
            let dragged_dir = match src_dir {
                Direction::Request => ContainerDirection::Request,
                Direction::Result => ContainerDirection::Result,
            };
            let geometry = self.engine.geometry();
            let (target_geom, over_dept) =
                if let Some(dst_id) = self.canvas.object_at_any(cur, &published.dynamic) {
                    let obj = self.canvas.objects.iter().find(|o| o.id == dst_id);
                    match obj.map(|o| o.kind) {
                        Some(polygram_core::types::NodeKind::Department) => {
                            (None, Some(dst_id))
                        }
                        _ => (obj.cloned(), None),
                    }
                } else if under_any {
                    self.canvas.objects
                        .iter()
                        .find(|o| {
                            o.id != src_id
                                && o.rect.x <= cur_cell.0
                                && cur_cell.0 < o.rect.x + o.rect.w
                                && o.rect.y <= cur_cell.1
                                && cur_cell.1 < o.rect.y + o.rect.h
                        })
                        .map(|o| (Some(o.clone()), None))
                        .unwrap_or((None, None))
                } else {
                    (None, None)
                };

            // Не соединять объект с самим собой.
            let is_self = target_geom.as_ref().map_or(false, |t| t.id == src_id);
            if !is_self {
                let cmds = compute_metric_drop_object(
                    &geometry,
                    &src_id,
                    &src_name,
                    dragged_dir,
                    cur_cell,
                    cur_cell,
                    over_dept.as_deref(),
                    (2, 2),
                    target_geom.as_ref(),
                );
                let mut applied = false;
                if !cmds.commands.is_empty() {
                    match self.engine.apply_batch(
                        polygram_core::types::Scope::Root,
                        cmds.commands,
                    ) {
                        Ok(_) => applied = true,
                        Err((_, e)) => {
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
                                let cmds2 = compute_metric_drop_object(
                                    &geometry,
                                    &src_id,
                                    &src_name,
                                    dragged_dir,
                                    cur_cell,
                                    cur_cell,
                                    over_dept.as_deref(),
                                    (2, 2),
                                    target_geom.as_ref(),
                                );
                                if !cmds2.commands.is_empty() {
                                    let _ = self.engine.apply_batch(
                                        polygram_core::types::Scope::Root,
                                        cmds2.commands,
                                    );
                                    applied = true;
                                }
                            }
                        }
                    }
                }
                if applied {
                    self.sync_canvas_from_engine();
                }
            }
        }
        true
    }
}

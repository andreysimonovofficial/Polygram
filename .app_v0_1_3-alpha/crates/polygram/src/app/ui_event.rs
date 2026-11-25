//! Обработка событий взаимодействия на холсте (CentralPanel).
//! Отвечает за:
//! - интеракцию с холстом (клик, драг, панорамирование, selection box);
//! - начало/конец перетаскивания объектов;
//! - драг-показателя и реордер/соединение показателей;
//! - выделение рамкой.
//! Диалоги после панели обрабатываются в mod.rs (ui()).

use egui::{Pos2, PointerButton, Rect, Vec2};
use polygram_core::{
    drag_domain::{compute_metric_drop, compute_metric_drop_object, MetricDropContext},
    types::{ContainerDirection, Direction},
};
use polygram_canvas::paint::object::process_total_height;
use crate::app::PolygramApp;
use polygram_core::runtime::Published;

impl PolygramApp {
    /// Обработать весь блок CentralPanel: интеракцию с холстом,
    /// перетаскивание объектов, панорамирование камеры, выделение рамкой.
    /// Вызывается из `ui()` внутри `egui::CentralPanel::show()`.
    pub(crate) fn handle_canvas_panel(&mut self, ui: &mut egui::Ui, published: &Published) {
        let canvas_response = ui.interact(
            ui.clip_rect(),
            ui.scope_id().with("canvas"),
            egui::Sense::click_and_drag(),
        );

        self.canvas.handle_camera(ui.ctx(), &canvas_response, ui);

        // Геометрия с учётом перетаскивания, кэшируется в простое.
        let geometry = if self.drag_objects.is_empty() {
            if let Some(ref cached) = self.cached_geometry {
                cached.clone()
            } else {
                let geometry = crate::app::departments::geometry_with_predictions(
                    &self.canvas.objects,
                    None,
                    None,
                );
                self.cached_geometry = Some(geometry.clone());
                geometry
            }
        } else {
            crate::app::departments::geometry_with_predictions(
                &self.canvas.objects,
                self.drag_objects.first().map(|s| s.as_str()),
                self.drag_over.as_deref(),
            )
        };
        self.canvas.objects = geometry.objects;

        // Плавное визуальное смещение перетаскиваемых объектов (пиксели экрана).
        // rect остаётся привязанным к сетке; визуально объект сдвигается на
        // фракцию клетки, чтобы двигаться плавно, а не прыгать по клеткам.
        let cell_px = self.canvas.cell_px();
        let drag_shift = self
            .drag_visual_offset
            .map(|(ox, oy)| egui::Vec2::new(ox * cell_px, oy * cell_px));
        let drag_ids = &self.drag_visual_ids;
        self.canvas.paint(
            ui,
            &published.dynamic,
            self.right_click_object.as_deref(),
            &self.selected_objects,
            drag_shift.map(|shift| (shift, drag_ids)),
        );

        if !canvas_response.hovered() {
            super::interaction::context_menu(self, ui.ctx(), published, self.canvas.theme);
            super::interaction::rename_dialog(self, ui.ctx());
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
                    self.dialogs.add_metric_process_name = self.canvas.objects.iter()
                        .find(|o| o.id == *obj_id)
                        .map(|o| o.name.clone())
                        .unwrap_or_default();
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
                    // Шапка контейнера (включая шапку внутри карточка процесса) — драг контейнера.
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
                    if self.canvas.objects.iter().any(|o| o.id == obj_id && o.kind == polygram_core::types::NodeKind::Process) {
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
                        // Драг контейнера/показателя как объекта (включая шапку контейнера внутри карточка процесса).
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
                    // Драг контейнера/показателя как объекта (включая шапку контейнера внутри карточка процесса).
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
                        }
                        self.drag_metric_source = None;
                        return;
                    }
                }
                // Соединение показателя с объектом-целью: ищем любой объект под курсором.
                if let Some(dst_id) = self.canvas.object_at_any(cur, &published.dynamic) {
                    if src_id != dst_id {
                        let geometry = self.engine.geometry();
                        let cmds = compute_metric_drop(
                            &MetricDropContext {
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
                                        compute_metric_drop(
                                            &MetricDropContext {
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
                    let cmds = compute_metric_drop_object(
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
                        let rect_px = if object.kind == polygram_core::types::NodeKind::Process {
                            let zoom = self.canvas.camera.zoom;
                            let card_w = 512.0 * zoom;
                            let card_h = process_total_height(
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
    }
}

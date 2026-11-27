//! Обработка событий взаимодействия на холсте (CentralPanel).
//! Отвечает за:
//! - интеракцию с холстом (клик, драг, панорамирование, selection box);
//! - начало/конец перетаскивания объектов;
//! - драг-показателя и реордер/соединение показателей;
//! - выделение рамкой.
//! Диалоги после панели обрабатываются в mod.rs (ui()).
//!
//! Декомпозирован по модулям:
//! - `click_handling` — клики: выделение, добавление показателя, контекстное меню, переименование
//! - `metric_drag`    — перетаскивание показателя: соединение, реордер
//! - `selection_box`  — выделение рамкой

use egui::PointerButton;
use polygram_core::runtime::Published;
use crate::app::PolygramApp;

pub(crate) mod click_handling;
pub(crate) mod metric_drag;
pub(crate) mod selection_box;

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

        // Обработка кликов (вынесено в click_handling)
        self.handle_canvas_clicks(ui, published);

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
                    self.start_drag_metric(obj_id, dir, name, pos);
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
            // Selection box отрисовка (вынесено в selection_box)
            self.handle_selection_box(ui, published);

            // Драг показателя — отрисовка линии (вынесено в metric_drag)
            let _ = self.handle_metric_drag(ui, published);
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

        // Завершение перетаскивания показателя (вынесено в metric_drag)
        if self.drag_metric_source.is_some()
            && !ui.input(|i| i.pointer.button_down(PointerButton::Primary))
        {
            let _ = self.handle_metric_drag(ui, published);
        }
    }
}

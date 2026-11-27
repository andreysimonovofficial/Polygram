//! Обработка кликов на холсте: выделение, добавление показателя, контекстное меню, переименование.

use egui::{PointerButton};
use polygram_core::types::Direction;
use polygram_core::runtime::Published;
use crate::app::PolygramApp;

impl PolygramApp {
    /// Обработать клики по холсту: ПКМ контекстное меню, ЛКМ выделение,
    /// клик на "+ Добавить показатель", двойной клик переименование.
    pub(crate) fn handle_canvas_clicks(
        &mut self,
        ui: &egui::Ui,
        published: &Published,
    ) {
        // ПКМ: контекстное меню на отделе/процессе или пустом месте
        if ui.input(|i| i.pointer.secondary_clicked()) {
            if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                self.selection_box_start = None;
                self.right_click = Some(pos);
                self.right_click_object = self.process_under(pos, &published.dynamic)
                    .or_else(|| self.object_under(pos));
                self.menu_just_opened = true;
            }
        }

        // ЛКМ клик: выделение объекта
        if ui.input(|i| i.pointer.primary_clicked()) {
            if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
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

        // Двойной клик по процессу — инлайн-переименование
        if ui.input(|i| i.pointer.button_double_clicked(PointerButton::Primary)) {
            if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
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
    }
}

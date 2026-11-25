//! Управление выделением объектов.
//! Отвечает за:
//! - toggle_select — добавить/убрать объект из выделения (с учетом Ctrl).

use crate::app::PolygramApp;

impl PolygramApp {
    /// Выделить объект (добавить, если уже выделен — убрать при Ctrl).
    pub(crate) fn toggle_select(&mut self, id: String, ctrl: bool) {
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

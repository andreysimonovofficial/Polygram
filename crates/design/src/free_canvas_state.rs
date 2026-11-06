//! Состояние свободного холста: загрузка/сохранение настроек, next_id, грязный флаг.

use crate::design_format::FreeCanvasDoc;
use crate::free_canvas::FreeCanvas;

impl FreeCanvas {
    /// Загрузить настройки и последний проект из хранилища eframe.
    pub(crate) fn load_settings(&mut self, storage: &dyn eframe::Storage) {
        if let Some(s) = storage.get_string("polygram_design/sidebar_width") {
            if let Ok(w) = s.parse::<f32>() {
                self.sidebar_width = w.clamp(200.0, 1000.0);
            }
        }
        if let Some(json) = storage.get_string("polygram_design/last_doc") {
            if let Ok(doc) = FreeCanvasDoc::load_from_json(&json) {
                self.doc = doc;
                self.recompute_next_id();
            }
        }
    }

    /// Сохранить настройки и последний проект в хранилище eframe.
    pub(crate) fn save_settings(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string("polygram_design/sidebar_width", self.sidebar_width.to_string());
        storage.set_string("polygram_design/last_doc", self.doc.to_json().unwrap_or_default());
    }

    /// Пересчитать next_id на основе загруженных фигур.
    pub(crate) fn recompute_next_id(&mut self) {
        let mut max: u64 = 0;
        for shape in &self.doc.shapes {
            if let Some(num) = shape.id.strip_prefix("shape-") {
                if let Ok(n) = num.parse::<u64>() {
                    if n > max {
                        max = n;
                    }
                }
            }
        }
        self.next_id = max + 1;
    }

    /// Отметить документ изменённым (для автосохранения).
    pub(crate) fn touch(&mut self) {
        self.doc_dirty = true;
    }
}

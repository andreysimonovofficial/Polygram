use std::collections::HashMap;

use polygram_core::{EntryGeom, types::Direction};

pub struct DragState {
    pub drag_object: Option<String>,
    pub drag_start_screen: Option<egui::Pos2>,
    pub drag_start_cell: Option<(i32, i32)>,
    pub predicted_positions: HashMap<String, (i32, i32)>,
    pub pending_move: Option<(String, u64)>,
    /// Возраст pending_move в кадрах — для отката при отклонении сервером (CellOccupied).
    pub pending_move_age: u32,
    /// Начало выделения рамкой (Selection Box) в экранных координатах.
    pub selection_box_start: Option<egui::Pos2>,
    /// Перетаскивание показателя: (id объекта-источника, направление, копия строки показателя).
    pub drag_metric: Option<(String, Direction, EntryGeom)>,
    pub drag_metric_start_screen: Option<egui::Pos2>,
    /// Отдел, под которым началось перетаскивание объекта. Его рамка растягивается
    /// под перетаскиваемый объект «на лету» (до подтверждения сервером).
    pub drag_department: Option<String>,
    /// Свободная клетка, куда объект уедет при Drop (призрачная копия / пунктирная рамка).
    pub drag_ghost: Option<(i32, i32)>,
    /// Объект или отдел, под которым сейчас нависает перетаскиваемый процесс.
    pub drag_over: Option<String>,
}

impl DragState {
    pub fn new() -> Self {
        Self {
            drag_object: None,
            drag_start_screen: None,
            drag_start_cell: None,
            predicted_positions: HashMap::new(),
            pending_move: None,
            pending_move_age: 0,
            selection_box_start: None,
            drag_metric: None,
            drag_metric_start_screen: None,
            drag_department: None,
            drag_ghost: None,
            drag_over: None,
        }
    }

    pub fn start_drag(&mut self, object_id: String, start_screen: egui::Pos2, start_cell: (i32, i32)) {
        self.drag_object = Some(object_id);
        self.drag_start_screen = Some(start_screen);
        self.drag_start_cell = Some(start_cell);
    }

    pub fn end_drag(&mut self) -> Option<(String, (i32, i32), Option<(i32, i32)>)> {
        let object_id = self.drag_object.take()?;
        self.drag_department = None;
        self.drag_ghost = None;
        self.drag_over = None;
        let start_cell = self.drag_start_cell.take()?;
        self.drag_start_screen = None;
        let target = self.predicted_positions.get(&object_id).copied();
        Some((object_id, start_cell, target))
    }

    pub fn update_prediction(&mut self, object_id: &str, target: (i32, i32)) {
        self.predicted_positions.insert(object_id.to_owned(), target);
    }

    pub fn clear_prediction(&mut self, object_id: &str) {
        self.predicted_positions.remove(object_id);
    }

    pub fn is_dragging(&self) -> bool {
        self.drag_object.is_some() || self.drag_metric.is_some()
    }
}

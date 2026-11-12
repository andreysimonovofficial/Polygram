use std::collections::HashMap;

use egui::Pos2;
use polygram_core::{EntryGeom, types::Direction};

/// Конечный автомат взаимодействия с графом на холсте.
/// Каждый вариант — явное состояние взаимодействия.
#[derive(Clone, Debug, PartialEq)]
pub enum DragInteractionState {
    /// Ничего не происходит.
    Idle,
    /// Перетаскивание объекта за шапку.
    DraggingObject {
        object_id: String,
        start_screen: Pos2,
        start_cell: (i32, i32),
    },
    /// Перетаскивание показателя (метрики) из одного объекта в другой.
    DraggingMetric {
        source_object: String,
        source_dir: Direction,
        entry: EntryGeom,
        start_screen: Pos2,
    },
    /// Выделение рамкой (selection box).
    SelectingBox {
        start_pos: Pos2,
    },
    /// Объект отпущен, решается куда девать (Move / ReparentAndMove / Cancel).
    DroppingObject {
        object_id: String,
        start_cell: (i32, i32),
        target: Option<(i32, i32)>,
    },
    /// Показатель отпущен, решается целевой объект и связь.
    DroppingMetric {
        source_object: String,
        source_dir: Direction,
        entry: EntryGeom,
    },
}

pub struct DragState {
    /// Текущее состояние конечного автомата.
    pub state: DragInteractionState,
    /// Предсказанные позиции объектов (клиентское предсказание, пока сервер не подтвердил).
    pub predicted_positions: HashMap<String, (i32, i32)>,
    /// Ожидающий подтверждения MoveObject (для отката при CellOccupied).
    pub pending_move: Option<(String, u64)>,
    /// Возраст pending_move в кадрах.
    pub pending_move_age: u32,
    /// Свободная клетка для призрачной копии при дропе объекта.
    pub drag_ghost: Option<(i32, i32)>,
    /// Отдел, над которым нависает перетаскиваемый объект.
    pub drag_over: Option<String>,
}

impl DragState {
    pub fn new() -> Self {
        Self {
            state: DragInteractionState::Idle,
            predicted_positions: HashMap::new(),
            pending_move: None,
            pending_move_age: 0,
            drag_ghost: None,
            drag_over: None,
        }
    }

    // ---------- совместимость с существующим кодом (доступ к полям) ----------

    /// ID перетаскиваемого объекта (если состояние DraggingObject или DroppingObject).
    pub fn drag_object(&self) -> Option<&str> {
        match &self.state {
            DragInteractionState::DraggingObject { object_id, .. }
            | DragInteractionState::DroppingObject { object_id, .. } => Some(object_id),
            _ => None,
        }
    }

    /// Стартовая экранная позиция перетаскивания.
    pub fn drag_start_screen(&self) -> Option<Pos2> {
        match &self.state {
            DragInteractionState::DraggingObject { start_screen, .. }
            | DragInteractionState::DraggingMetric { start_screen, .. } => {
                Some(*start_screen)
            }
            _ => None,
        }
    }

    /// Стартовая клетка перетаскиваемого объекта.
    pub fn drag_start_cell(&self) -> Option<(i32, i32)> {
        match &self.state {
            DragInteractionState::DraggingObject { start_cell, .. }
            | DragInteractionState::DroppingObject { start_cell, .. } => Some(*start_cell),
            _ => None,
        }
    }

    /// Начало выделения рамкой.
    pub fn selection_box_start(&self) -> Option<Pos2> {
        match &self.state {
            DragInteractionState::SelectingBox { start_pos } => Some(*start_pos),
            _ => None,
        }
    }

    /// Перетаскиваемый показатель.
    pub fn drag_metric(&self) -> Option<(String, Direction, EntryGeom)> {
        match &self.state {
            DragInteractionState::DraggingMetric {
                source_object,
                source_dir,
                entry,
                ..
            }
            | DragInteractionState::DroppingMetric {
                source_object,
                source_dir,
                entry,
            } => Some((source_object.clone(), *source_dir, entry.clone())),
            _ => None,
        }
    }

    /// Стартовая позиция перетаскивания показателя.
    pub fn drag_metric_start_screen(&self) -> Option<Pos2> {
        match &self.state {
            DragInteractionState::DraggingMetric { start_screen, .. } => Some(*start_screen),
            _ => None,
        }
    }

    // ---------- новые методы FSM ----------

    pub fn is_dragging(&self) -> bool {
        matches!(
            &self.state,
            DragInteractionState::DraggingObject { .. }
                | DragInteractionState::DraggingMetric { .. }
        )
    }

    pub fn is_selecting(&self) -> bool {
        matches!(&self.state, DragInteractionState::SelectingBox { .. })
    }

    pub fn is_dropping(&self) -> bool {
        matches!(
            &self.state,
            DragInteractionState::DroppingObject { .. }
                | DragInteractionState::DroppingMetric { .. }
        )
    }

    pub fn start_drag_object(
        &mut self,
        object_id: String,
        start_screen: Pos2,
        start_cell: (i32, i32),
    ) {
        self.state = DragInteractionState::DraggingObject {
            object_id,
            start_screen,
            start_cell,
        };
        self.drag_ghost = None;
        self.drag_over = None;
    }

    pub fn start_drag_metric(
        &mut self,
        source_object: String,
        source_dir: Direction,
        entry: EntryGeom,
        start_screen: Pos2,
    ) {
        self.state = DragInteractionState::DraggingMetric {
            source_object,
            source_dir,
            entry,
            start_screen,
        };
    }

    pub fn start_selection_box(&mut self, start_pos: Pos2) {
        self.state = DragInteractionState::SelectingBox { start_pos };
    }

    /// Перевести в состояние DroppingObject, зафиксировав цель.
    pub fn begin_drop_object(&mut self, target: Option<(i32, i32)>) {
        let (object_id, start_cell) = match std::mem::replace(
            &mut self.state,
            DragInteractionState::Idle,
        ) {
            DragInteractionState::DraggingObject {
                object_id, start_cell, ..
            } => (object_id, start_cell),
            other => {
                self.state = other;
                return;
            }
        };
        self.state = DragInteractionState::DroppingObject {
            object_id,
            start_cell,
            target,
        };
    }

    /// Завершить перетаскивание объекта: вернуть (id, стартовая клетка, цель).
    pub fn end_drag_object(&mut self) -> Option<(String, (i32, i32), Option<(i32, i32)>)> {
        let (object_id, start_cell, target) = match std::mem::replace(
            &mut self.state,
            DragInteractionState::Idle,
        ) {
            DragInteractionState::DroppingObject {
                object_id,
                start_cell,
                target,
            } => (object_id, start_cell, target),
            DragInteractionState::DraggingObject {
                object_id,
                start_cell,
                ..
            } => (object_id, start_cell, None),
            other => {
                self.state = other;
                return None;
            }
        };
        self.drag_ghost = None;
        self.drag_over = None;
        Some((object_id, start_cell, target))
    }

    /// Перевести в состояние DroppingMetric.
    pub fn begin_drop_metric(&mut self) {
        if let DragInteractionState::DraggingMetric {
            source_object,
            source_dir,
            entry,
            ..
        } = std::mem::replace(&mut self.state, DragInteractionState::Idle)
        {
            self.state = DragInteractionState::DroppingMetric {
                source_object,
                source_dir,
                entry,
            };
        }
    }

    /// Завершить перетаскивание показателя: вернуть (объект, направление, запись).
    pub fn end_drag_metric(&mut self) -> Option<(String, Direction, EntryGeom)> {
        match std::mem::replace(&mut self.state, DragInteractionState::Idle) {
            DragInteractionState::DroppingMetric {
                source_object,
                source_dir,
                entry,
            } => Some((source_object, source_dir, entry)),
            DragInteractionState::DraggingMetric {
                source_object,
                source_dir,
                entry,
                ..
            } => Some((source_object, source_dir, entry)),
            other => {
                self.state = other;
                None
            }
        }
    }

    /// Отменить любое активное взаимодействие, вернуться в Idle.
    pub fn cancel(&mut self) {
        self.state = DragInteractionState::Idle;
        self.drag_ghost = None;
        self.drag_over = None;
    }

    pub fn update_prediction(&mut self, object_id: &str, target: (i32, i32)) {
        self.predicted_positions.insert(object_id.to_owned(), target);
    }

    pub fn clear_prediction(&mut self, object_id: &str) {
        self.predicted_positions.remove(object_id);
    }
}

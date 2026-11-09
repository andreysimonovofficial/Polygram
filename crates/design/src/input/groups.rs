//! Группировка, порядок слоёв (передний/задний план) и удаление фигур.

use crate::free_canvas::FreeCanvas;

impl FreeCanvas {
    /// Сгруппировать выделенные фигуры (присвоить общий group_id).
    pub(crate) fn group_selected(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        self.save_undo_state();
        let group_id = format!("group-{}", self.next_id);
        self.next_id += 1;
        for &idx in &self.selected {
            if let Some(shape) = self.doc.shapes.get_mut(idx) {
                shape.group_id = Some(group_id.clone());
            }
        }
        self.touch();
    }

    /// Разгруппировать выделенные фигуры (убрать group_id).
    pub(crate) fn ungroup_selected(&mut self) {
        self.save_undo_state();
        for &idx in &self.selected {
            if let Some(shape) = self.doc.shapes.get_mut(idx) {
                shape.group_id = None;
            }
        }
        self.touch();
    }

    /// Переместить выделенные фигуры на передний план (в конец списка shapes).
    pub(crate) fn send_to_front(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        self.save_undo_state();
        let selected_ids: Vec<String> = self.selected.iter()
            .filter_map(|&i| self.doc.shapes.get(i).map(|s| s.id.clone()))
            .collect();
        let mut selected_shapes: Vec<crate::design_format::FreeShape> = Vec::new();
        let mut remaining: Vec<crate::design_format::FreeShape> = Vec::new();
        for (i, shape) in self.doc.shapes.drain(..).enumerate() {
            if self.selected.contains(&i) {
                selected_shapes.push(shape);
            } else {
                remaining.push(shape);
            }
        }
        remaining.extend(selected_shapes);
        self.doc.shapes = remaining;
        self.selected = self.doc.shapes.iter()
            .enumerate()
            .filter_map(|(i, s)| if selected_ids.contains(&s.id) { Some(i) } else { None })
            .collect();
        self.touch();
    }

    /// Переместить выделенные фигуры на задний план (в начало списка shapes).
    pub(crate) fn send_to_back(&mut self) {
        if self.selected.is_empty() {
            return;
        }
        self.save_undo_state();
        let selected_ids: Vec<String> = self.selected.iter()
            .filter_map(|&i| self.doc.shapes.get(i).map(|s| s.id.clone()))
            .collect();
        let mut selected_shapes: Vec<crate::design_format::FreeShape> = Vec::new();
        let mut remaining: Vec<crate::design_format::FreeShape> = Vec::new();
        for (i, shape) in self.doc.shapes.drain(..).enumerate() {
            if self.selected.contains(&i) {
                selected_shapes.push(shape);
            } else {
                remaining.push(shape);
            }
        }
        let mut new_shapes = selected_shapes;
        new_shapes.extend(remaining);
        self.doc.shapes = new_shapes;
        self.selected = self.doc.shapes.iter()
            .enumerate()
            .filter_map(|(i, s)| if selected_ids.contains(&s.id) { Some(i) } else { None })
            .collect();
        self.touch();
    }

    /// Удалить все выделенные фигуры.
    pub(crate) fn delete_selected(&mut self) {
        self.save_undo_state();
        let mut keep: Vec<crate::design_format::FreeShape> = Vec::new();
        for (i, shape) in self.doc.shapes.drain(..).enumerate() {
            if !self.selected.contains(&i) {
                keep.push(shape);
            }
        }
        self.doc.shapes = keep;
        self.selected.clear();
        self.context_menu = None;
        self.hovered.clear();
        self.pressed.clear();
        self.direct_hover = None;
        self.direct_press = None;
        self.recompute_next_id();
        self.touch();
    }
}

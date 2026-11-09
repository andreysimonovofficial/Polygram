//! Прямой hit-тест, активное множество (цель + дети + группа), иерархия containment.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::free_canvas::FreeCanvas;
use crate::design_format::ChildReactMode;

impl FreeCanvas {
    /// Прямой topmost hit без распространения (верхняя фигура под курсором).
    pub(crate) fn direct_hit(&self, world: egui::Pos2) -> Option<usize> {
        self.hit_shape(world)
    }

    /// Активное множество: цель + её дети (по иерархии containment) + группа (горизонтально).
    pub(crate) fn active_set(&self, hit: Option<usize>) -> HashSet<usize> {
        let mut set = HashSet::new();
        let Some(idx) = hit else { return set };

        set.insert(idx);

        // Группировка по group_id (горизонтальная связь, существовавшая раньше).
        if let Some(gid) = &self.doc.shapes[idx].group_id {
            for (j, s) in self.doc.shapes.iter().enumerate() {
                if s.group_id.as_ref() == Some(gid) {
                    set.insert(j);
                }
            }
        }

        // Распространение вниз по дереву containment (родитель -> дети).
        if self.is_container(idx) && self.child_react_mode(idx) != ChildReactMode::None {
            self.collect_descendants(idx, &mut set);
        }

        set
    }

    /// Все подходящие потомки (рекурсивно вниз).
    fn collect_descendants(&self, idx: usize, out: &mut HashSet<usize>) {
        let parent_id = &self.doc.shapes[idx].id;
        let mode = self.child_react_mode(idx);
        let allow = &self.doc.shapes[idx].child_react_allow;

        for (j, child) in self.doc.shapes.iter().enumerate() {
            if child.parent_id.as_ref() != Some(parent_id) {
                continue;
            }

            let allowed = match mode {
                ChildReactMode::All => child.reacts_to_parent,
                ChildReactMode::List => child.reacts_to_parent && allow.contains(&child.id),
                ChildReactMode::None => false,
            };

            if allowed {
                out.insert(j);
                self.collect_descendants(j, out);
            }
        }
    }

    /// Является ли фигура рамкой-контейнером.
    fn is_container(&self, idx: usize) -> bool {
        self.doc.shapes.get(idx).map(|s| s.is_container).unwrap_or(false)
    }

    /// Режим распространения состояния на детей.
    fn child_react_mode(&self, idx: usize) -> ChildReactMode {
        self.doc.shapes
            .get(idx)
            .map(|s| s.child_react_mode)
            .unwrap_or(ChildReactMode::All)
    }

    /// Проверить, не создаст ли установка родителя цикл в иерархии.
    fn would_create_cycle(&self, idx: usize, new_parent_id: &str) -> bool {
        let mut cur = self
            .doc
            .shapes
            .iter()
            .position(|s| s.id == new_parent_id);
        while let Some(p) = cur {
            if p == idx {
                return true;
            }
            cur = self.doc.shapes[p]
                .parent_id
                .as_ref()
                .and_then(|id| self.doc.shapes.iter().position(|s| s.id == *id));
        }
        false
    }

    /// Перестроить порядок фигур так, чтобы родитель шел раньше всех потомков.
    /// Необходимо для корректного hit-теста (обратный обход должен находить ребёнка раньше родителя).
    pub(crate) fn reindex_containment(&mut self) {
        let n = self.doc.shapes.len();
        let id_to_idx: HashMap<String, usize> = self
            .doc
            .shapes
            .iter()
            .enumerate()
            .map(|(i, s)| (s.id.clone(), i))
            .collect();

        let mut parent_idx: Vec<Option<usize>> = vec![None; n];
        for (i, s) in self.doc.shapes.iter().enumerate() {
            if let Some(pid) = &s.parent_id {
                parent_idx[i] = id_to_idx.get(pid).copied();
            }
        }

        // Topological sort: родитель před ребёнком (Kahn's algorithm).
        let mut remaining_deps: Vec<usize> = vec![0; n];
        for i in 0..n {
            if parent_idx[i].is_some() {
                remaining_deps[i] = 1;
            }
        }

        let mut queue: VecDeque<usize> = (0..n).filter(|&i| remaining_deps[i] == 0).collect();
        let mut order = Vec::with_capacity(n);

        while let Some(i) = queue.pop_front() {
            order.push(i);
            for j in 0..n {
                if parent_idx[j] == Some(i) {
                    remaining_deps[j] -= 1;
                    if remaining_deps[j] == 0 {
                        queue.push_back(j);
                    }
                }
            }
        }

        // Остальные (сироты с пропавшим родителем или циклы) — в конец.
        for i in 0..n {
            if !order.contains(&i) {
                order.push(i);
            }
        }

        // Запомнить старые id по старым индексам до замены вектора.
        let old_ids: Vec<String> = self.doc.shapes.iter().map(|s| s.id.clone()).collect();
        let new_shapes: Vec<crate::design_format::FreeShape> = order.into_iter().map(|i| self.doc.shapes[i].clone()).collect();
        let id_to_new_idx: HashMap<&str, usize> = new_shapes
            .iter()
            .enumerate()
            .map(|(new_i, s)| (s.id.as_str(), new_i))
            .collect();
        // Перенаправить выделенные индексы на новые позиции по id фигур.
        self.selected.retain_mut(|idx| {
            let old_id = old_ids.get(*idx).map(|s| s.as_str()).unwrap_or("");
            id_to_new_idx.get(old_id).copied().map_or(false, |new_i| { *idx = new_i; true })
        });
        self.doc.shapes = new_shapes;
    }
}

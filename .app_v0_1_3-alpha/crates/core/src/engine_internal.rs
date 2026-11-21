//! Внутренние хелперы Engine: растяжение и сжатие отделов под содержимое.

use crate::{
    event::EventKind,
    types::*,
};

impl crate::engine::Engine {
    /// Растянуть отдел под всех своих детей (с отступом в одну клетку сетки с каждой стороны,
    /// заголовок в верхней клетке, дети начинаются на строке parent.y + 2).
    /// Итеративно поднимается от `start` к корню, чтобы стек не рос с глубиной вложенности.
    pub(crate) fn expand_department_to_fit(&mut self, start: ObjectIx) {
        let mut chain: Vec<ObjectIx> = Vec::new();
        let mut cur = start;
        loop {
            chain.push(cur);
            match self.hier.parent_of(cur) {
                Some(p) => cur = p,
                None => break,
            }
        }
        for &anc in &chain {
            let children: Vec<ObjectIx> = self.hier.children_of(anc);
            if children.is_empty() {
                continue;
            }
            let mut min_x = i32::MAX;
            let mut min_y = i32::MAX;
            let mut max_x = i32::MIN;
            let mut max_y = i32::MIN;
            for &c in &children {
                let r = self.layout.rect_of(c);
                min_x = min_x.min(r.x);
                min_y = min_y.min(r.y);
                max_x = max_x.max(r.x + r.w);
                max_y = max_y.max(r.y + r.h);
            }
            let new_y = min_y - 2;
            let new_x = min_x - 1;
            let new_w = (max_x + 1) - new_x;
            let new_h = (max_y + 1) - new_y;
            let new_rect = crate::layout::Rect::new(new_x, new_y, new_w, new_h);
            let old = self.layout.rect_of(anc);
            if new_rect == old {
                continue;
            }
            self.layout.replace(anc, new_rect);
            self.pending.push(EventKind::ObjectMoved {
                object: anc.0,
                gx: new_x,
                gy: new_y,
            });
        }
    }

    /// Сжать отдел под оставшихся детей.
    /// Итеративно поднимается к корню.
    pub(crate) fn shrink_department_to_fit(&mut self, start: ObjectIx) {
        let mut chain: Vec<ObjectIx> = Vec::new();
        let mut cur = start;
        loop {
            chain.push(cur);
            match self.hier.parent_of(cur) {
                Some(p) => cur = p,
                None => break,
            }
        }
        for &anc in &chain {
            let children: Vec<ObjectIx> = self.hier.children_of(anc);
            let old = self.layout.rect_of(anc);
            if children.is_empty() {
                continue;
            }
            let mut min_x = i32::MAX;
            let mut min_y = i32::MAX;
            let mut max_x = i32::MIN;
            let mut max_y = i32::MIN;
            for &c in &children {
                let r = self.layout.rect_of(c);
                min_x = min_x.min(r.x);
                min_y = min_y.min(r.y);
                max_x = max_x.max(r.x + r.w);
                max_y = max_y.max(r.y + r.h);
            }
            let new_y = min_y - 2;
            let new_x = min_x - 1;
            let new_w = (max_x + 1) - new_x;
            let new_h = (max_y + 1) - new_y;
            let new_rect = crate::layout::Rect::new(new_x, new_y, new_w, new_h);
            if new_rect == old {
                continue;
            }
            self.layout.replace(anc, new_rect);
            self.pending.push(EventKind::ObjectMoved {
                object: anc.0,
                gx: new_x,
                gy: new_y,
            });
        }
    }
}

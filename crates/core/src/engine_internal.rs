//! Внутренние хелперы Engine: поиск объектов, синхронизация хранилищ, маршруты, рефит.

use crate::{
    error::{EngineError, Result},
    event::EventKind,
    types::*,
};

impl crate::engine::Engine {
    pub(crate) fn find_object(&self, id: &str) -> Result<ObjectIx> {
        self.hier
            .find_object(id)
            .ok_or_else(|| EngineError::ObjectNotFound(id.to_owned()))
    }

    pub(crate) fn find_entry(&self, object: &str, entry: &str) -> Result<(ObjectIx, EntryIx)> {
        let o = self.find_object(object)?;
        let e = self
            .hier
            .find_entry(o, entry)
            .ok_or_else(|| EngineError::MetricNotFound {
                object: object.to_owned(),
                metric: entry.to_owned(),
            })?;
        Ok((o, e))
    }

    pub(crate) fn find_metric_def(&self, id: &str) -> Result<MetricDefIx> {
        self.hier
            .find_metric_def(id)
            .ok_or_else(|| EngineError::ObjectNotFound(id.to_owned()))
    }

    pub(crate) fn sync_sizes(&mut self) {
        self.vals
            .ensure(self.hier.object_metric_slots(), self.hier.edge_slots());
        self.layout
            .ensure(self.hier.object_slots());
    }

    pub(crate) fn remove_edge_internal(&mut self, edge: crate::types::EdgeIx) {
        let d = *self.hier.edge(edge);
        self.hier.disconnect(edge);
        self.vals.alloc[edge.idx()] = 0.0;
        self.vals.mark_out(d.from);
        self.vals.mark_in(d.to);
        self.pending.push(EventKind::EdgeRemoved { edge: edge.0 });
    }

    pub(crate) fn plan_refit(
        &self,
        o: ObjectIx,
        n_req: usize,
        n_out: usize,
    ) -> Result<Option<crate::layout::Rect>> {
        let (w, h) = crate::layout::footprint_for(n_req, n_out);
        let (w, h) = (w as i32, h as i32);
        let r = self.layout.rect_of(o);
        if r.w == w && r.h == h {
            return Ok(None);
        }
        let grown = crate::layout::Rect::new(r.x, r.y, w, h);
        if self.layout.is_free(&grown, Some(o)) {
            return Ok(Some(grown));
        }
        self.layout
            .find_free_near(w, h, r.x, r.y, Some(o), 256)
            .map(|(x, y)| Some(crate::layout::Rect::new(x, y, w, h)))
            .ok_or(EngineError::NoRoom)
    }

    pub(crate) fn apply_refit(&mut self, o: ObjectIx, plan: Option<crate::layout::Rect>) {
        if let Some(new) = plan {
            let old = self.layout.rect_of(o);
            self.layout.replace(o, new);
            if (new.x, new.y) != (old.x, old.y) {
                self.pending.push(EventKind::ObjectMoved {
                    object: o.0,
                    gx: new.x,
                    gy: new.y,
                });
            }
        }
    }

    /// Растянуть отдел под всех своих детей (с отступом в одну клетку сетки с каждой стороны,
    /// заголовок в верхней клетке, дети начинаются на строке parent.y + 2).
    /// Итеративно поднимается от `start` к корню, чтобы стек не рос с глубиной вложенности.
    pub(crate) fn expand_department_to_fit(&mut self, mut start: ObjectIx) -> Option<crate::layout::Rect> {
        let mut chain: Vec<ObjectIx> = Vec::new();
        let mut cur = start;
        loop {
            chain.push(cur);
            match self.hier.parent_of(cur) {
                Some(p) => cur = p,
                None => break,
            }
        }
        let mut last_rect: Option<crate::layout::Rect> = None;
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
            last_rect = Some(new_rect);
        }
        last_rect
    }

    /// Сжать отдел под оставшихся детей (если возможно без наложений).
    /// Итеративно поднимается к корню.
    pub(crate) fn shrink_department_to_fit(&mut self, mut start: ObjectIx) -> Option<crate::layout::Rect> {
        let mut chain: Vec<ObjectIx> = Vec::new();
        let mut cur = start;
        loop {
            chain.push(cur);
            match self.hier.parent_of(cur) {
                Some(p) => cur = p,
                None => break,
            }
        }
        let mut last_rect: Option<crate::layout::Rect> = None;
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
            last_rect = Some(new_rect);
        }
        last_rect
    }
}

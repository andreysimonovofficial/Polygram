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
            .ensure(self.hier.object_slots(), self.hier.edge_slots());
    }

    pub(crate) fn queue_entry_routes(&mut self, e: EntryIx) {
        for edge in self.hier.out_entries(e) {
            self.layout.queue_route(edge);
        }
        for edge in self.hier.in_entries(e) {
            self.layout.queue_route(edge);
        }
    }

    pub(crate) fn remove_edge_internal(&mut self, edge: crate::types::EdgeIx) {
        let d = *self.hier.edge(edge);
        self.hier.disconnect(edge);
        self.vals.alloc[edge.idx()] = 0.0;
        self.vals.mark_out(d.from);
        self.vals.mark_in(d.to);
        self.layout.release_route(edge);
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
            let reqs: Vec<crate::types::EntryIx> = self.hier.object(o).requests.clone();
            let outs: Vec<crate::types::EntryIx> = self.hier.object(o).outputs.clone();
            for &e in &reqs {
                self.queue_entry_routes(e);
            }
            for &e in &outs {
                self.queue_entry_routes(e);
            }
        }
    }
}

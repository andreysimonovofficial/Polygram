//! Движок — единственный владелец графа.
//! Три хранилища: Иерархия (структура) · Значения/статусы (быстро меняющееся) · Архитектура графа (положение и линии).
//! Все изменения идут командами (`Command`); все результаты — события (`Event`) с номером ревизии.

use crate::{
    bottleneck::{self, Bottleneck},
    command::*,
    commit::finish,
    error::{EngineError, Result},
    event::{EventGap, EventKind},
    event_subjects::event_subjects,
    geometry::{EdgeGeom, EntryGeom, Geometry, ObjectGeom},
    hierarchy::Hierarchy,
    layout::{footprint_for, Layout, Rect},
    types::*,
    values::{DynamicState, Values},
    views::{MetricDefView, MetricView, ObjectView, Stats},
};
use std::collections::HashSet;

pub(crate) fn same_num(a: f64, b: f64) -> bool {
    (a.is_nan() && b.is_nan()) || a == b
}

#[derive(Clone, Debug)]
pub struct Engine {
    pub(crate) name: String,
    pub(crate) hier: Hierarchy,
    pub(crate) vals: Values,
    pub(crate) layout: Layout,
    pub(crate) log: crate::event::EventLog,
    pub(crate) rev: u64,
    pub(crate) geo_rev: u64,
    pub(crate) pending: Vec<EventKind>,
    pub(crate) scratch_changes: Vec<(crate::types::EntryIx, StatusCode, StatusCode)>,
}

const EVENT_LOG_CAPACITY: usize = 200_000;

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        Self::with_capacity(0, 0)
    }

    pub fn with_capacity(objects: usize, edges: usize) -> Self {
        Self {
            name: "Новый граф".to_owned(),
            hier: Hierarchy::with_capacity(objects, edges),
            vals: Values::new(),
            layout: Layout::new(),
            log: crate::event::EventLog::new(EVENT_LOG_CAPACITY),
            rev: 0,
            geo_rev: 0,
            pending: Vec::new(),
            scratch_changes: Vec::new(),
        }
    }

    // ---------- чтение ----------
    pub fn rev(&self) -> u64 {
        self.rev
    }
    pub fn geometry_rev(&self) -> u64 {
        self.geo_rev
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }
    pub fn hierarchy(&self) -> &Hierarchy {
        &self.hier
    }
    pub fn values(&self) -> &Values {
        &self.vals
    }
    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    pub fn stats(&self) -> Stats {
        Stats {
            objects: self.hier.object_count(),
            metric_defs: self.hier.metric_def_count(),
            object_metrics: self.hier.object_metric_count(),
            edges: self.hier.edge_count(),
            rev: self.rev,
        }
    }

    pub fn events_since(
        &self,
        rev: u64,
    ) -> std::result::Result<Vec<crate::event::Event>, EventGap> {
        self.log.since(rev)
    }

    /// Все объекты в поддереве отдела (включая сам отдел).
    fn subtree_objects(&self, dept: ObjectIx) -> Vec<ObjectIx> {
        let mut out = Vec::new();
        let mut stack = vec![dept];
        while let Some(cur) = stack.pop() {
            out.push(cur);
            for child in self.hier.children_of(cur) {
                stack.push(child);
            }
        }
        out
    }

    /// События отдела: о собственных объектах отдела и всех его дочерних отделах.
    pub fn department_events(&self, dept_ext_id: &str) -> Result<Vec<crate::event::Event>> {
        let dept_ix = self
            .hier
            .find_object(dept_ext_id)
            .ok_or_else(|| EngineError::ObjectNotFound(dept_ext_id.to_owned()))?;
        let subtree: HashSet<ObjectIx> = self.subtree_objects(dept_ix).into_iter().collect();
        Ok(self
            .log
            .iter()
            .filter(|e| {
                event_subjects(&e.kind, &self.hier)
                    .iter()
                    .any(|s| subtree.contains(s))
            })
            .cloned()
            .collect())
    }

    pub fn object_ids(&self) -> Vec<String> {
        self.hier
            .live_objects()
            .map(|o| self.hier.object(o).ext_id.clone())
            .collect()
    }

    pub fn metric_def_ids(&self) -> Vec<String> {
        self.hier
            .live_metric_defs()
            .map(|m| self.hier.metric_def(m).ext_id.clone())
            .collect()
    }

    pub fn metric_view(&self, e: EntryIx) -> MetricView {
        let om = self.hier.object_metric(e);
        let md = self.hier.metric_def(om.metric_def);
        let i = e.idx();
        let v = &self.vals;
        let opt = |x: f64| if x.is_nan() { None } else { Some(x) };
        MetricView {
            id: om.ext_id.clone(),
            name: om.ext_id.clone(),
            metric_def_id: md.ext_id.clone(),
            metric_def_name: md.name.clone(),
            unit: md.unit.clone(),
            kind: md.kind,
            direction: om.direction,
            value: v.value[i],
            supplied: v.supplied[i],
            required: v.required[i],
            max: opt(v.max[i]),
            fact: v.fact[i],
            connected: self.hier.has_edges(e),
            status: v.status[i],
            status_text: v.status[i].label_ru(),
        }
    }

    pub fn object_view(&self, ext: &str) -> Option<ObjectView> {
        let ix = self.hier.find_object(ext)?;
        let o = self.hier.object(ix);
        let r = self.layout.rect_of(ix);
        Some(ObjectView {
            id: o.ext_id.clone(),
            name: o.name.clone(),
            kind: o.kind,
            at: [r.x, r.y],
            size: [r.w, r.h],
            requests: o.requests.iter().map(|&e| self.metric_view(e)).collect(),
            outputs: o.outputs.iter().map(|&e| self.metric_view(e)).collect(),
        })
    }

    pub fn metric_def_view(&self, ext: &str) -> Option<MetricDefView> {
        let ix = self.hier.find_metric_def(ext)?;
        let md = self.hier.metric_def(ix);
        Some(MetricDefView {
            id: md.ext_id.clone(),
            name: md.name.clone(),
            kind: md.kind,
            unit: md.unit.clone(),
            api: md.api.clone(),
        })
    }

    pub fn dynamic_state(&self) -> DynamicState {
        self.vals.snapshot(self.rev)
    }

    pub fn geometry(&self) -> Geometry {
        let objects = self
            .hier
            .live_objects()
            .map(|o| {
                let d = self.hier.object(o);
                let requests: Vec<EntryGeom> = d
                    .requests
                    .iter()
                    .map(|&e| {
                        let i = e.idx();
                        EntryGeom {
                            ix: e.0,
                            name: self.hier.object_metric(e).ext_id.clone(),
                            value: self.vals.supplied[i],
                            supplied: self.vals.supplied[i],
                        }
                    })
                    .collect();
                let outputs: Vec<EntryGeom> = d
                    .outputs
                    .iter()
                    .map(|&e| {
                        let i = e.idx();
                        EntryGeom {
                            ix: e.0,
                            name: self.hier.object_metric(e).ext_id.clone(),
                            value: self.vals.value[i],
                            supplied: self.vals.value[i],
                        }
                    })
                    .collect();
                ObjectGeom {
                    ix: o.0,
                    id: d.ext_id.clone(),
                    name: d.name.clone(),
                    kind: d.kind,
                    rect: self.layout.rect_of(o),
                    requests,
                    outputs,
                }
            })
            .collect();
        let edges = self
            .hier
            .live_edges()
            .map(|e| {
                let d = self.hier.edge(e);
                EdgeGeom {
                    ix: e.0,
                    from: d.from.0,
                    to: d.to.0,
                    points: self.layout.route(e).to_vec(),
                }
            })
            .collect();
        Geometry {
            rev: self.geo_rev,
            objects,
            edges,
        }
    }

    pub fn bottlenecks(&self, limit: usize) -> Vec<Bottleneck> {
        bottleneck::find(&self.hier, &self.vals, limit)
    }

    pub fn can_place(&self, object: &str, at: [i32; 2]) -> Result<bool> {
        let ix = self.find_object(object)?;
        let r = self.layout.rect_of(ix);
        Ok(self
            .layout
            .is_free(&Rect::new(at[0], at[1], r.w, r.h), Some(ix)))
    }

    pub fn recomputed_state(&self) -> DynamicState {
        let mut copy = self.clone();
        let mut changes = Vec::new();
        copy.vals.recompute_all(&copy.hier, &mut changes);
        copy.vals.snapshot(copy.rev)
    }

    // ---------- команды ----------
    pub fn apply(&mut self, _scope: Scope, cmd: Command) -> Result<Applied> {
        let result = crate::commands::exec(self, cmd);
        finish(self);
        result.map(|mut a| {
            a.rev = self.rev;
            a
        })
    }

    pub fn apply_batch(
        &mut self,
        _scope: Scope,
        cmds: Vec<Command>,
    ) -> std::result::Result<Vec<Applied>, (usize, EngineError)> {
        let mut out = Vec::with_capacity(cmds.len());
        for (i, cmd) in cmds.into_iter().enumerate() {
            match crate::commands::exec(self, cmd) {
                Ok(a) => out.push(a),
                Err(e) => {
                    finish(self);
                    return Err((i, e));
                }
            }
        }
        finish(self);
        for a in &mut out {
            a.rev = self.rev;
        }
        Ok(out)
    }

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
    ) -> Result<Option<Rect>> {
        let (w, h) = footprint_for(n_req, n_out);
        let (w, h) = (w as i32, h as i32);
        let r = self.layout.rect_of(o);
        if r.w == w && r.h == h {
            return Ok(None);
        }
        let grown = Rect::new(r.x, r.y, w, h);
        if self.layout.is_free(&grown, Some(o)) {
            return Ok(Some(grown));
        }
        self.layout
            .find_free_near(w, h, r.x, r.y, Some(o), 256)
            .map(|(x, y)| Some(Rect::new(x, y, w, h)))
            .ok_or(EngineError::NoRoom)
    }

    pub(crate) fn apply_refit(&mut self, o: ObjectIx, plan: Option<Rect>) {
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
            // Перекладываем все линии объекта.
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

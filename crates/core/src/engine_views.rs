//! Представления Engine для чтения: метрики, объекты, геометрия, узкие места.

use crate::{
    bottleneck::{self, Bottleneck},
    event_subjects::event_subjects,
    types::*,
    values::DynamicState,
    views::{MetricDefView, MetricView, ObjectView, Stats},
};
use std::collections::HashSet;

impl crate::engine::Engine {
    pub fn stats(&self) -> Stats {
        Stats {
            objects: self.hier.object_count(),
            metric_defs: self.hier.metric_def_count(),
            object_metrics: self.hier.object_metric_count(),
            edges: self.hier.edge_count(),
            rev: self.rev,
        }
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

    pub fn metric_def_views(&self) -> Vec<MetricDefView> {
        self.hier
            .live_metric_defs()
            .map(|m| self.metric_def_view(&self.hier.metric_def(m).ext_id))
            .filter_map(|o| o)
            .collect()
    }

    pub fn metric_view(&self, e: EntryIx) -> MetricView {
        let om = self.hier.object_metric(e);
        let md = self.hier.metric_def(om.metric_def);
        let i = e.idx();
        let v = &self.vals;
        MetricView {
            id: om.ext_id.clone(),
            name: om.ext_id.clone(),
            metric_def_id: md.ext_id.clone(),
            metric_def_name: md.name.clone(),
            unit: md.unit.clone(),
            kind: md.kind,
            direction: om.direction,
            value: v.value[i].clone(),
            supplied: v.supplied[i].clone(),
            required: v.required[i].clone(),
            max: v.max[i].clone(),
            fact: v.fact[i].clone(),
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

    pub fn geometry(&self) -> crate::geometry::Geometry {
        let objects = self
            .hier
            .live_objects()
            .map(|o| {
                let d = self.hier.object(o);
                    let requests: Vec<crate::geometry::EntryGeom> = d
                        .requests
                        .iter()
                        .map(|&e| {
                            let i = e.idx();
                            let om = self.hier.object_metric(e);
                            let md = self.hier.metric_def(om.metric_def);
                            crate::geometry::EntryGeom {
                                ix: e.0,
                                name: om.ext_id.clone(),
                                value: self.vals.value[i].clone(),
                                supplied: self.vals.supplied[i].clone(),
                                unit: md.unit.clone(),
                            }
                        })
                        .collect();
                    let outputs: Vec<crate::geometry::EntryGeom> = d
                        .outputs
                        .iter()
                        .map(|&e| {
                            let i = e.idx();
                            let om = self.hier.object_metric(e);
                            let md = self.hier.metric_def(om.metric_def);
                            crate::geometry::EntryGeom {
                                ix: e.0,
                                name: om.ext_id.clone(),
                                value: self.vals.value[i].clone(),
                                supplied: self.vals.supplied[i].clone(),
                                unit: md.unit.clone(),
                            }
                        })
                    .collect();
                crate::geometry::ObjectGeom {
                    ix: o.0,
                    id: d.ext_id.clone(),
                    name: d.name.clone(),
                    kind: d.kind,
                    rect: self.layout.rect_of(o),
                    requests,
                    outputs,
                    parent: d.parent.map(|p| self.hier.object(p).ext_id.clone()),
                }
            })
            .collect();
        let edges = self
            .hier
            .live_edges()
            .map(|e| {
                let d = self.hier.edge(e);
                crate::geometry::EdgeGeom {
                    ix: e.0,
                    from: d.from.0,
                    to: d.to.0,
                    points: self.layout.route(e).to_vec(),
                }
            })
            .collect();
        crate::geometry::Geometry {
            rev: self.geo_rev,
            objects,
            edges,
        }
    }

    pub fn bottlenecks(&self, limit: usize) -> Vec<Bottleneck> {
        bottleneck::find(&self.hier, &self.vals, limit)
    }

    pub fn can_place(&self, object: &str, at: [i32; 2]) -> crate::error::Result<bool> {
        let ix = self.find_object(object)?;
        let r = self.layout.rect_of(ix);
        Ok(self
            .layout
            .is_free(&crate::layout::Rect::new(at[0], at[1], r.w, r.h), Some(ix)))
    }

    /// Все объекты в поддереве отдела (включая сам отдел).
    pub(crate) fn subtree_objects(&self, dept: ObjectIx) -> Vec<ObjectIx> {
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
    pub fn department_events(&self, dept_ext_id: &str) -> crate::error::Result<Vec<crate::event::Event>> {
        let dept_ix = self
            .hier
            .find_object(dept_ext_id)
            .ok_or_else(|| crate::error::EngineError::ObjectNotFound(dept_ext_id.to_owned()))?;
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

    pub fn recomputed_state(&self) -> DynamicState {
        let mut copy = self.clone();
        let mut changes = Vec::new();
        copy.vals.recompute_all(&copy.hier, &mut changes);
        copy.vals.snapshot(copy.rev)
    }
}

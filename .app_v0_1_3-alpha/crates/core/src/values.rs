//! Хранилище значений и статусов записей контейнеров.

use crate::{hierarchy::Hierarchy, types::*};
use chrono::Utc;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct Values {
    pub value: Vec<MetricValue>,
    pub supplied: Vec<MetricValue>,
    pub required: Vec<MetricValue>,
    pub max: Vec<Option<MetricValue>>,
    pub fact: Vec<MetricValue>,
    pub status: Vec<StatusCode>,
    pub dirty_out: Vec<bool>,
    pub dirty_in: Vec<bool>,
    pub out_list: Vec<EntryIx>,
    pub in_list: Vec<EntryIx>,
    pub alloc: Vec<f64>,
}

impl Values {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ensure(&mut self, object_metrics: usize, edges: usize) {
        if self.value.len() < object_metrics {
            let old = self.value.len();
            self.value.resize(object_metrics, MetricValue::Null);
            self.supplied.resize(object_metrics, MetricValue::Null);
            self.required.resize(object_metrics, MetricValue::Null);
            self.max.resize(object_metrics, None);
            self.fact.resize(object_metrics, MetricValue::Null);
            self.status.resize(object_metrics, StatusCode::Ok);
            self.dirty_out.resize(object_metrics, false);
            self.dirty_in.resize(object_metrics, false);
            for i in old..object_metrics {
                self.out_list.push(EntryIx(i as u32));
                self.in_list.push(EntryIx(i as u32));
            }
        }
        if self.alloc.len() < edges {
            self.alloc.resize(edges, 0.0);
        }
    }

    pub fn reset_metric(&mut self, e: EntryIx) {
        let i = e.idx();
        self.value[i] = MetricValue::Null;
        self.supplied[i] = MetricValue::Null;
        self.required[i] = MetricValue::Null;
        self.max[i] = None;
        self.fact[i] = MetricValue::Null;
        self.status[i] = StatusCode::Ok;
    }

    pub fn mark_out(&mut self, e: EntryIx) {
        self.dirty_out[e.idx()] = true;
        self.out_list.push(e);
    }

    pub fn mark_in(&mut self, e: EntryIx) {
        self.dirty_in[e.idx()] = true;
        self.in_list.push(e);
    }

    pub fn mark_metric(&mut self, h: &Hierarchy, e: EntryIx) {
        let meta = h.object_metric(e);
        match meta.direction {
            Direction::Result => self.mark_out(e),
            Direction::Request => self.mark_in(e),
        }
    }

    pub fn mark_sources_of(&mut self, h: &Hierarchy, request_entry: EntryIx) {
        for edge in h.in_entries(request_entry) {
            let from = h.edge(edge).from;
            self.mark_out(from);
        }
    }

    pub fn recompute_all(
        &mut self,
        h: &Hierarchy,
        changes: &mut Vec<(EntryIx, StatusCode, StatusCode)>,
    ) {
        changes.clear();
        for e in h.live_object_metrics() {
            let meta = h.object_metric(e);
            if meta.direction == Direction::Result {
                self.recompute_exit(h, e, changes);
            }
        }
        for e in h.live_object_metrics() {
            let meta = h.object_metric(e);
            if meta.direction == Direction::Request {
                self.recompute_request(h, e, changes);
            }
        }
    }

    pub fn settle(&mut self, h: &Hierarchy, changes: &mut Vec<(EntryIx, StatusCode, StatusCode)>) {
        changes.clear();
        let out_list: Vec<EntryIx> = self.out_list.drain(..).collect();
        for &e in &out_list {
            if self.dirty_out[e.idx()] {
                self.dirty_out[e.idx()] = false;
                self.recompute_exit(h, e, changes);
            }
        }
        let in_list: Vec<EntryIx> = self.in_list.drain(..).collect();
        for &e in &in_list {
            if self.dirty_in[e.idx()] {
                self.dirty_in[e.idx()] = false;
                self.recompute_request(h, e, changes);
            }
        }
    }

    fn metric_kind(&self, h: &Hierarchy, e: EntryIx) -> MetricKind {
        let meta = h.object_metric(e);
        h.metric_def(meta.metric_def).kind
    }

    fn recompute_exit(
        &mut self,
        h: &Hierarchy,
        e: EntryIx,
        changes: &mut Vec<(EntryIx, StatusCode, StatusCode)>,
    ) {
        let meta = h.object_metric(e);
        debug_assert_eq!(meta.direction, Direction::Result);
        let i = e.idx();
        let kind = self.metric_kind(h, e);

        let new_status = match kind {
            MetricKind::Number | MetricKind::Time | MetricKind::Percentage | MetricKind::Money | MetricKind::Duration => {
                let result_value = match self.value[i] {
                    MetricValue::Number(v) => v,
                    MetricValue::Percentage(v) => v,
                    MetricValue::Money(v) => v as f64,
                    MetricValue::Duration(v) => v as f64,
                    _ => 0.0,
                };

                let mut sum_requests: f64 = 0.0;
                for edge in h.out_entries(e) {
                    let to = h.edge(edge).to;
                    let req_meta = h.object_metric(to);
                    if req_meta.direction == Direction::Request {
                        let t = h.edge(edge).throughput;
                        let req_val = match self.required[to.idx()] {
                            MetricValue::Number(v) => v,
                            MetricValue::Percentage(v) => v,
                            MetricValue::Money(v) => v as f64,
                            MetricValue::Duration(v) => v as f64,
                            _ => 0.0,
                        };
                        sum_requests += req_val * (t / 100.0);
                    }
                }

                if !h.has_edges(e) {
                    StatusCode::Surplus
                } else if sum_requests > result_value + EPS {
                    if h.object_metric(e).in_process {
                        StatusCode::InProgress
                    } else {
                        StatusCode::Expected
                    }
                } else if (sum_requests - result_value).abs() <= EPS {
                    StatusCode::Ok
                } else {
                    StatusCode::Surplus
                }
            }
            MetricKind::Deadline => {
                let now = Utc::now();
                match self.value[i] {
                    MetricValue::Deadline(dt) | MetricValue::Date(dt) => {
                        if dt < now {
                            StatusCode::Expected
                        } else {
                            StatusCode::Ok
                        }
                    }
                    _ => StatusCode::Ok,
                }
            }
            _ => StatusCode::Ok,
        };

        if new_status != self.status[i] {
            let old = self.status[i];
            self.status[i] = new_status;
            changes.push((e, old, new_status));
        }
    }

    fn recompute_request(
        &mut self,
        h: &Hierarchy,
        e: EntryIx,
        changes: &mut Vec<(EntryIx, StatusCode, StatusCode)>,
    ) {
        let meta = h.object_metric(e);
        debug_assert_eq!(meta.direction, Direction::Request);
        let i = e.idx();
        let kind = self.metric_kind(h, e);

        let new_status = match kind {
            MetricKind::Number | MetricKind::Time | MetricKind::Percentage | MetricKind::Money | MetricKind::Duration => {
                let required = match self.required[i] {
                    MetricValue::Number(v) => v,
                    MetricValue::Percentage(v) => v,
                    MetricValue::Money(v) => v as f64,
                    MetricValue::Duration(v) => v as f64,
                    _ => 0.0,
                };

                let mut sum_supply: f64 = 0.0;
                for edge in h.in_entries(e) {
                    let from = h.edge(edge).from;
                    let src_meta = h.object_metric(from);
                    if src_meta.direction == Direction::Result {
                        let t = h.edge(edge).throughput;
                        let val = match self.value[from.idx()] {
                            MetricValue::Number(v) => v,
                            MetricValue::Percentage(v) => v,
                            MetricValue::Money(v) => v as f64,
                            MetricValue::Duration(v) => v as f64,
                            _ => 0.0,
                        };
                        sum_supply += val * (t / 100.0);
                    }
                }
                self.supplied[i] = MetricValue::Number(sum_supply.min(required));

                if sum_supply >= required - EPS {
                    StatusCode::Ok
                } else {
                    StatusCode::Expected
                }
            }
            _ => StatusCode::Ok,
        };

        if new_status != self.status[i] {
            let old = self.status[i];
            self.status[i] = new_status;
            changes.push((e, old, new_status));
        }
    }

    pub fn snapshot(&self, rev: u64) -> DynamicState {
        DynamicState {
            rev,
            value: self.value.clone(),
            required: self.required.clone(),
            max: self.max.clone(),
            fact: self.fact.clone(),
            status: self.status.clone(),
        }
    }
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DynamicState {
    pub rev: u64,
    pub value: Vec<MetricValue>,
    pub required: Vec<MetricValue>,
    pub max: Vec<Option<MetricValue>>,
    pub fact: Vec<MetricValue>,
    pub status: Vec<StatusCode>,
}

impl PartialEq for DynamicState {
    fn eq(&self, other: &Self) -> bool {
        self.rev == other.rev
            && self.value.len() == other.value.len()
            && self.value == other.value
            && self.required == other.required
            && self.max == other.max
            && self.fact == other.fact
            && self.status == other.status
    }
}

impl Eq for DynamicState {}

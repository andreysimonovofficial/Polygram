//! Хранилище 2 — Значения и статусы записей контейнеров.
//! Значения хранятся по записи (объект + запись показателя), а не по глобальному показателю.
//! Один и тот же глобальный показатель может иметь разные значения в разных объектах.

use crate::{hierarchy::Hierarchy, types::*};
use chrono::Utc;

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct Values {
    /// Значение записи (per-object).
    pub value: Vec<MetricValue>,
    /// Поставлено источниками (per-object, для запросов — сумма подключённых выходов).
    pub supplied: Vec<MetricValue>,
    /// Необходимое значение (per-object, для запросов).
    pub required: Vec<MetricValue>,
    /// Максимум (per-object).
    pub max: Vec<Option<MetricValue>>,
    /// Факт (фактическое значение, per-object).
    pub fact: Vec<MetricValue>,
    /// Статус записи.
    pub status: Vec<StatusCode>,
    /// Грязные флаги для пересчёта.
    pub dirty_out: Vec<bool>,
    pub dirty_in: Vec<bool>,
    /// Список записей, требующих пересчёта.
    pub out_list: Vec<EntryIx>,
    pub in_list: Vec<EntryIx>,
    /// Выделенное под рёбра (alloc).
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
            self.status.resize(object_metrics, StatusCode::Unknown);
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
        self.status[i] = StatusCode::Unknown;
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
            Direction::Output => self.mark_out(e),
            Direction::Request => self.mark_in(e),
        }
    }

    pub fn mark_sources_of(&mut self, h: &Hierarchy, request_entry: EntryIx) {
        // Для запроса: помечаем все источники (выходы, подключённые к этому запросу)
        for edge in h.in_entries(request_entry) {
            let from = h.edge(edge).from;
            self.mark_out(from);
        }
    }

    /// Пересчитать все значения и статусы.
    pub fn recompute_all(
        &mut self,
        h: &Hierarchy,
        changes: &mut Vec<(EntryIx, StatusCode, StatusCode)>,
    ) {
        changes.clear();
        // Сначала выходы (источники), потом запросы (цели).
        for e in h.live_object_metrics() {
            let meta = h.object_metric(e);
            if meta.direction == Direction::Output {
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

    /// Инкрементальный пересчёт помеченных записей.
    pub fn settle(&mut self, h: &Hierarchy, changes: &mut Vec<(EntryIx, StatusCode, StatusCode)>) {
        changes.clear();
        // Выходы сначала.
        let out_list: Vec<EntryIx> = self.out_list.drain(..).collect();
        for &e in &out_list {
            if self.dirty_out[e.idx()] {
                self.dirty_out[e.idx()] = false;
                self.recompute_exit(h, e, changes);
            }
        }
        // Потом запросы.
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
        debug_assert_eq!(meta.direction, Direction::Output);
        let i = e.idx();
        let kind = self.metric_kind(h, e);

        let new_status = match kind {
            MetricKind::Number | MetricKind::Time | MetricKind::Percentage | MetricKind::Money | MetricKind::Duration => {
                let exit_value = match self.value[i] {
                    MetricValue::Number(v) => v,
                    MetricValue::Percentage(v) => v,
                    MetricValue::Money(v) => v as f64,
                    MetricValue::Duration(v) => v as f64,
                    _ => 0.0,
                };

                // Сумма запросов целей (взвешенная пропускной способностью).
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
                    StatusCode::Unconnected
                } else if sum_requests > exit_value + EPS {
                    StatusCode::Deficit
                } else if (sum_requests - exit_value).abs() <= EPS {
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
                            StatusCode::Deficit // просрочено
                        } else if (dt - now).num_seconds() < 86400 {
                            StatusCode::Warning // до конца дня
                        } else {
                            StatusCode::Ok // в срок
                        }
                    }
                    _ => StatusCode::Unknown,
                }
            }
            MetricKind::Date => {
                if self.value[i].is_null() {
                    StatusCode::Unknown
                } else {
                    StatusCode::Ok
                }
            }
            MetricKind::Binary => match self.value[i] {
                MetricValue::Binary(true) => StatusCode::Ok,
                MetricValue::Binary(false) => StatusCode::Invalid,
                _ => StatusCode::Unknown,
            },
            _ => StatusCode::Unknown,
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

                // Сумма выходов источников (взвешенная пропускной способностью).
                let mut sum_supply: f64 = 0.0;
                for edge in h.in_entries(e) {
                    let from = h.edge(edge).from;
                    let src_meta = h.object_metric(from);
                    if src_meta.direction == Direction::Output {
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

                if !h.has_edges(e) {
                    StatusCode::Unconnected
                } else if sum_supply >= required - EPS {
                    StatusCode::Ok
                } else {
                    StatusCode::Deficit
                }
            }
            MetricKind::Deadline => {
                let now = Utc::now();
                match self.required[i] {
                    MetricValue::Deadline(dt) | MetricValue::Date(dt) => {
                        if dt < now {
                            StatusCode::Deficit
                        } else if (dt - now).num_seconds() < 86400 {
                            StatusCode::Warning
                        } else {
                            StatusCode::Ok
                        }
                    }
                    _ => StatusCode::Unknown,
                }
            }
            MetricKind::Date => {
                if self.required[i].is_null() {
                    StatusCode::Unknown
                } else {
                    StatusCode::Ok
                }
            }
            _ => StatusCode::Unknown,
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

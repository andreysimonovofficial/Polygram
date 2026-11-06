//! Хранилище 2 — Значения и статусы записей контейнеров.
//! Значения хранятся по записи (объект + запись показателя), а не по глобальному показателю.
//! Один и тот же глобальный показатель может иметь разные значения в разных объектах.

use crate::{hierarchy::Hierarchy, types::*};

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct Values {
    /// Значение записи (per-object).
    pub value: Vec<f64>,
    /// Поставлено источниками (per-object, для запросов — сумма подключённых выходов).
    pub supplied: Vec<f64>,
    /// Необходимое значение (per-object, для запросов).
    pub required: Vec<f64>,
    /// Максимум (per-object).
    pub max: Vec<f64>,
    /// Факт (фактическое значение, per-object).
    pub fact: Vec<f64>,
    /// Статус записи.
    pub status: Vec<StatusCode>,
    /// Грязные флаги для пересчёта.
    pub dirty_out: Vec<bool>,
    pub dirty_in: Vec<bool>,
    /// Список записей, требующих пересчёта.
    pub out_list: Vec<EntryIx>,
    pub in_list: Vec<EntryIx>,
    /// Выделенное под рёбер (alloc).
    pub alloc: Vec<f64>,
}

impl Values {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ensure(&mut self, object_metrics: usize, edges: usize) {
        if self.value.len() < object_metrics {
            let old = self.value.len();
            self.value.resize(object_metrics, 0.0);
            self.supplied.resize(object_metrics, 0.0);
            self.required.resize(object_metrics, 0.0);
            self.max.resize(object_metrics, f64::NAN);
            self.fact.resize(object_metrics, 0.0);
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
        self.value[i] = 0.0;
        self.supplied[i] = 0.0;
        self.required[i] = 0.0;
        self.max[i] = f64::NAN;
        self.fact[i] = 0.0;
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

    fn recompute_exit(
        &mut self,
        h: &Hierarchy,
        e: EntryIx,
        changes: &mut Vec<(EntryIx, StatusCode, StatusCode)>,
    ) {
        let meta = h.object_metric(e);
        debug_assert_eq!(meta.direction, Direction::Output);
        let i = e.idx();
        let exit_value = self.value[i];

        // Сумма запросов целей (взвешенная пропускной способностью).
        let mut sum_requests: f64 = 0.0;
        for edge in h.out_entries(e) {
            let to = h.edge(edge).to;
            let req_meta = h.object_metric(to);
            if req_meta.direction == Direction::Request {
                let t = h.edge(edge).throughput;
                sum_requests += self.required[to.idx()] * (t / 100.0);
            }
        }

        let new_status = if !h.has_edges(e) {
            StatusCode::Unconnected
        } else if sum_requests > exit_value + EPS {
            StatusCode::Deficit
        } else if (sum_requests - exit_value).abs() <= EPS {
            StatusCode::Ok
        } else {
            StatusCode::Surplus
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
        let required = self.required[i];

        // Сумма выходов источников (взвешенная пропускной способностью).
        let mut sum_supply: f64 = 0.0;
        for edge in h.in_entries(e) {
            let from = h.edge(edge).from;
            let src_meta = h.object_metric(from);
            if src_meta.direction == Direction::Output {
                let t = h.edge(edge).throughput;
                sum_supply += self.value[from.idx()] * (t / 100.0);
            }
        }
        self.supplied[i] = sum_supply.min(required);

        let new_status = if !h.has_edges(e) {
            StatusCode::Unconnected
        } else if sum_supply >= required - EPS {
            StatusCode::Ok
        } else {
            StatusCode::Deficit
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

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct DynamicState {
    pub rev: u64,
    pub value: Vec<f64>,
    pub required: Vec<f64>,
    pub max: Vec<f64>,
    pub fact: Vec<f64>,
    pub status: Vec<StatusCode>,
}

impl PartialEq for DynamicState {
    fn eq(&self, other: &Self) -> bool {
        self.rev == other.rev
            && self.value.len() == other.value.len()
            && self
                .value
                .iter()
                .zip(other.value.iter())
                .all(|(a, b)| (a - b).abs() < 1e-9)
            && self
                .required
                .iter()
                .zip(other.required.iter())
                .all(|(a, b)| (a - b).abs() < 1e-9)
            && self
                .max
                .iter()
                .zip(other.max.iter())
                .all(|(a, b)| (a.is_nan() && b.is_nan()) || (a - b).abs() < 1e-9)
            && self
                .fact
                .iter()
                .zip(other.fact.iter())
                .all(|(a, b)| (a - b).abs() < 1e-9)
            && self.status == other.status
    }
}

impl Eq for DynamicState {}

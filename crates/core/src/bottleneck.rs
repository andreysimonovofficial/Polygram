//! Определитель узких мест (правило v0).
//!
//! Узкое место — выход в дефиците: потребителям нужно больше, чем он производит.
//! Важность = недостача (спрос − значение); при равенстве — больше потребителей, затем меньший индекс.

use crate::{hierarchy::Hierarchy, types::*, values::Values};
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Bottleneck {
    pub object: u32,
    pub entry: u32,
    pub shortfall: f64,
    pub consumers: u32,
}

pub fn find(h: &Hierarchy, v: &Values, limit: usize) -> Vec<Bottleneck> {
    let mut found: Vec<Bottleneck> = h
        .live_object_metrics()
        .filter(|&e| {
            h.object_metric(e).direction == Direction::Output
                && v.status[e.idx()] == StatusCode::Deficit
        })
        .map(|e| {
            // Недостача = сумма требуемого целями − значение выхода.
            let mut demand: f64 = 0.0;
            for edge in h.out_entries(e) {
                let to = h.edge(edge).to;
                let to_meta = h.object_metric(to);
                if to_meta.direction == Direction::Request {
                    demand += v.required[to.idx()] * (h.edge(edge).throughput / 100.0);
                }
            }
            Bottleneck {
                object: h.object_metric(e).object.0,
                entry: e.0,
                shortfall: (demand - v.value[e.idx()]).max(0.0),
                consumers: h.out_entries(e).count() as u32,
            }
        })
        .collect();
    found.sort_by(|a, b| {
        b.shortfall
            .partial_cmp(&a.shortfall)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.consumers.cmp(&a.consumers))
            .then(a.entry.cmp(&b.entry))
    });
    found.truncate(limit);
    found
}

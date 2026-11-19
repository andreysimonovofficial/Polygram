//! Представления объектов и показателей для вывода.

use crate::types::{Direction, MetricKind, MetricValue, NodeKind, StatusCode};

#[derive(Clone, Debug, serde::Serialize)]
pub struct Stats {
    pub objects: usize,
    pub metric_defs: usize,
    pub object_metrics: usize,
    pub edges: usize,
    pub rev: u64,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct MetricView {
    pub id: String,
    pub name: String,
    pub metric_def_id: String,
    pub metric_def_name: String,
    pub unit: String,
    pub kind: MetricKind,
    pub direction: Direction,
    pub value: MetricValue,
    pub supplied: MetricValue,
    pub required: MetricValue,
    pub max: Option<MetricValue>,
    pub fact: MetricValue,
    pub connected: bool,
    pub status: StatusCode,
    pub status_text: &'static str,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct ObjectView {
    pub id: String,
    pub name: String,
    pub kind: NodeKind,
    pub at: [i32; 2],
    pub size: [i32; 2],
    pub requests: Vec<MetricView>,
    pub results: Vec<MetricView>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MetricDefView {
    pub id: String,
    pub name: String,
    pub kind: MetricKind,
    pub unit: String,
    pub api: Option<crate::types::MetricApi>,
}

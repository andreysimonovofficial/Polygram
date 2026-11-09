use crate::types::{Direction, MetricKind, NodeKind, StatusCode};

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
    pub value: f64,
    pub supplied: f64,
    pub required: f64,
    pub max: Option<f64>,
    pub fact: f64,
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
    pub outputs: Vec<MetricView>,
}

/// Глобальный показатель для вывода.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct MetricDefView {
    pub id: String,
    pub name: String,
    pub kind: MetricKind,
    pub unit: String,
    pub api: Option<crate::types::MetricApi>,
}

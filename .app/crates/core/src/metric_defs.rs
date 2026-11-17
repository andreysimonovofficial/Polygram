use crate::types::*;

/// Данные глобального показателя.
#[derive(Clone, Debug)]
pub struct MetricDefData {
    pub ext_id: String,
    pub name: String,
    pub kind: MetricKind,
    pub unit: String,
    pub api: Option<MetricApi>,
    pub alive: bool,
}

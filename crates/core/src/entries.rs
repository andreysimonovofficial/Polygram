use crate::types::*;

/// Данные записи показателя в контейнере объекта.
#[derive(Clone, Debug)]
pub struct ObjectMetricData {
    pub ext_id: String,
    pub object: ObjectIx,
    pub metric_def: MetricDefIx,
    pub direction: Direction,
    pub alive: bool,
}

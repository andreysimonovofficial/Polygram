//! Данные записи показателя в контейнере объекта.

use crate::types::*;

/// Данные записи показателя в контейнере объекта.
#[derive(Clone, Debug)]
pub struct ObjectMetricData {
    pub ext_id: String,
    pub object: ObjectIx,
    pub metric_def: MetricDefIx,
    pub direction: Direction,
    pub alive: bool,
    /// Флаг «В процессе» для записи Результата.
    pub in_process: bool,
}

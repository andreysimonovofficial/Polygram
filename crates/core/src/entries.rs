use crate::types::*;

/// Данные записи показателя в контейнере объекта.
#[derive(Clone, Debug)]
pub struct ObjectMetricData {
    pub ext_id: String,
    pub object: ObjectIx,
    pub metric_def: MetricDefIx,
    pub direction: Direction,
    pub alive: bool,
    /// Флаг «В процессе» для записи Выхода: если сумма запросов целей превышает значение выхода,
    /// статус зависит от этого флага — «В процессе» при активном флаге, иначе «Ожидается».
    pub in_process: bool,
}

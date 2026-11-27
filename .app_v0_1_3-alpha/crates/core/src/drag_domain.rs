//! Чистая доменная логика перетаскивания объектов и показателей на холсте.
//! Без egui, без окна, без сети. Только входное состояние и чистый результат.
//!
//! Декомпозирован по модулям (один файл — одна ответственность):
//! - `free_cell`       — поиск свободной клетки, целевой объект, противоположное направление
//! - `metric_drop`     — дроп показателя между объектами
//! - `object_drop`     — точка входа для дропа любого объекта
//! - `dept_drop`       — дроп отдела
//! - `process_drop`    — дроп процесса
//! - `container_drop`  — дроп контейнера (Запрос/Результат)
//! - `metric_drop_object` — дроп показателя как объекта

pub mod container_drop;
pub mod dept_drop;
pub mod free_cell;
pub mod metric_drop;
pub mod metric_drop_object;
pub mod object_drop;
pub mod process_drop;

pub use container_drop::ContainerDropResult;
pub use container_drop::compute_container_drop;
pub use dept_drop::DepartmentDropResult;
pub use dept_drop::compute_department_drop;
pub use free_cell::find_free_cell_in_dept_domain;
pub use free_cell::target_object_at;
pub use free_cell::opposite_dir;
pub use metric_drop::MetricDropContext;
pub use metric_drop::compute_metric_drop;
pub use metric_drop_object::MetricDropObjectResult;
pub use metric_drop_object::compute_metric_drop_object;
pub use object_drop::ObjectDropContext;
pub use object_drop::ObjectDropResult;
pub use object_drop::compute_object_drop_commands;
pub use process_drop::ProcessDropResult;
pub use process_drop::compute_process_drop;

//! Рендер объектов и линий связи на холсте.
//! Ответственность разнесена по файлам:
//! - `constants.rs` — геометрические константы.
//! - `status.rs` — ранжирование и агрегация статусов.
//! - `row.rs` — одна строка показателя, строка добавления, превью перетаскивания.
//! - `container.rs` — заголовок контейнера показателей.
//! - `object.rs` — объект "Процесс" и пунктирная рамка.
//! - `edge.rs` — цвет и вывод линии связи на экран (маршрут строит `routing`).

pub mod container;
pub mod constants;
pub mod edge;
pub mod object;
pub mod row;
pub mod status;

pub use edge::paint_edge;
pub use object::{paint_dashed_rect, paint_process};
pub use row::{MetricBinding, paint_drag_preview};

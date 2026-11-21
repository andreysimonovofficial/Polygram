//! Рендер объектов и линий связи на холсте.
//! Ответственность разнесена по файлам:
//! - `constants.rs` — геометрические константы.
//! - `status.rs` — ранжирование и агрегация статусов.
//! - `row.rs` — одна строка показателя, строка добавления.
//! - `container.rs` — заголовок контейнера показателей.
//! - `object.rs` — объект "Процесс" и пунктирная рамка.

pub mod container;
pub mod constants;
pub mod object;
pub mod row;
pub mod status;

pub use object::{paint_dashed_rect, paint_process, ProcessPaintResult};
pub use row::paint_add_row;

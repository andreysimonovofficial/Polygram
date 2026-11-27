//! Рендер объектов и линий связи на холсте.
//! Ответственность разнесена по файлам:
//! - `constants.rs` — геометрические константы.
//! - `status.rs` — ранжирование и агрегация статусов.
//! - `row.rs` — одна строка показателя, строка добавления.
//! - `container.rs` — заголовок контейнера показателей.
//! - `object.rs` — объект "Процесс" и пунктирная рамка.
//! - `cover.rs` — обложка процесса (когда не наведено).
//! - `process_header.rs` — шапка процесса.
//! - `edge.rs` — линии связи (цвет, скругление, отсечение).

pub mod container;
pub mod constants;
pub mod cover;
pub mod edge;
pub mod object;
pub mod process_header;
pub mod row;
pub mod status;

pub use object::{paint_process, ProcessPaintResult};
pub use row::paint_add_row;

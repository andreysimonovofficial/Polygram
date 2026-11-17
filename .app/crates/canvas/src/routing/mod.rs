//! Линии связи холста: маршрутизатор (чистая геометрия по клеткам сетки) и всё, что нужно,
//! чтобы показать результат на экране.
//!
//! * `model`, `cost`, `obstacles`, `stubs`, `direct`, `search`, `router` — алгоритм без egui;
//! * `layer` — кэш маршрутов холста и привязка к геометрии ядра;
//! * `ports` — точки сцепления строк показателей на экране;
//! * `screen` — перевод клеток в пиксели.

mod cost;
mod direct;
pub mod layer;
pub(crate) mod model;
mod obstacles;
pub mod ports;
mod router;
pub mod screen;
mod search;
mod stubs;
#[cfg(test)]
pub(crate) mod layer_tests;
#[cfg(test)]
mod tests;

pub use layer::EdgeLayer;
pub use model::{Route, Side};

//! Ортогональные линии связи между показателями (ТЗ «Ортогональные линии связи»).
//!
//! Модуль разделён на чистое ядро без зависимости от egui и тонкий слой отрисовки:
//! - `model`     — единицы, стороны, терминалы, маршрут, упрощение точек;
//! - `cost`      — веса стоимости (длина → колена → красота);
//! - `obstacles` — препятствия и проверки свободы отрезков;
//! - `stubs`     — допустимые колонки первого и последнего отрезков;
//! - `direct`    — прямая и «Z» без поиска;
//! - `search`    — A* по сжатой сетке;
//! - `router`    — ступени качества, область влияния, запасная ломаная;
//! - `ports`     — геометрия строк Процесса и точки сцепления;
//! - `layer`     — кэш маршрутов и ленивый пересчёт;
//! - `screen`    — перевод маршрута в экранные координаты.
//!
//! Линии нигде не сериализуются: они строятся из положения карточек.

pub mod cost;
pub mod direct;
pub mod layer;
pub mod model;
pub mod obstacles;
pub mod ports;
pub mod router;
pub mod screen;
pub mod search;
pub mod stubs;

#[cfg(test)]
mod layer_tests;
#[cfg(test)]
mod tests;

pub use layer::{CardInfo, EdgeLayer, EdgeRef, EntryRef};
pub use model::{Pt, Quality, Route, Side, Terminal, U};
pub use router::route;

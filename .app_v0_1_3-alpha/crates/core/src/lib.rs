//! Ядро вычислений Polygram: типы показателей, иерархия объектов,
//! контейнеры запросов/результатов, команды.
//!
//! Правила комнаты: ядро НЕ зависит от окна, сети и графики.
//! Всё, что меняет граф, — команда (`Command`); всё, что происходит, — событие с номером ревизии.

pub mod command;
pub mod commands;
pub mod commit;
pub mod edges;
pub mod engine;
pub mod entries;
pub mod event;
pub mod error;
pub mod geometry;
pub mod hierarchy;
pub mod hierarchy_mut;
pub mod layout;
pub mod metric_defs;
pub mod objects;
pub mod runtime;
pub mod types;
pub mod values;
pub mod views;

pub use command::{Applied, Command};
pub use engine::Engine;
pub use entries::ObjectMetricData;
pub use error::{EngineError, Result};
pub use geometry::{EdgeGeom, EntryGeom, Geometry, ObjectGeom};
pub use hierarchy::Hierarchy;
pub use layout::Rect;
pub use metric_defs::MetricDefData;
pub use runtime::Published;
pub use types::*;
pub use values::{DynamicState, Values};
pub use views::{MetricDefView, MetricView, ObjectView, Stats};

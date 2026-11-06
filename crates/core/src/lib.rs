//! Ядро вычислений Polygram.
//!
//! Правила комнаты: ядро НЕ зависит от окна, сети и графики. Его можно запускать на сервере,
//! в тестах и вベンчмарках. Всё, что меняет граф, — команда (`Command`); всё, что происходит, —
//! событие (`Event`) с номером ревизии.

pub mod autolayout;
pub mod bottleneck;
pub mod command;
pub mod commands;
pub mod commit;
pub mod document;
pub mod edges;
pub mod engine;
pub mod entries;
pub mod error;
pub mod event;
pub mod event_subjects;
pub mod geometry;
pub mod hierarchy;
pub mod layout;
pub mod metric_defs;
pub mod objects;
pub mod route;
pub mod runtime;
pub mod types;
pub mod values;
pub mod views;

pub use bottleneck::Bottleneck;
pub use command::{Applied, Command};
pub use commit::finish;
pub use document::{Document, FORMAT};
pub use engine::Engine;
pub use error::{EngineError, Result};
pub use event::{Event, EventGap, EventKind};
pub use geometry::{EdgeGeom, EntryGeom, Geometry, ObjectGeom};
pub use layout::{Point, Rect};
pub use runtime::{EngineHandle, EngineRuntime, Published};
pub use types::*;
pub use values::DynamicState;
pub use views::{MetricDefView, MetricView, ObjectView, Stats};

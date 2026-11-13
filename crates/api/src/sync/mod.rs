//! Модуль синхронизации показателей с внешними API.
//!
//! Ответственность разнесена по файлам:
//! - `source.rs` — трейт `MetricSource`, перечисление `SourceType`, реестр `SourceRegistry`.
//! - `static_source.rs` — источник «Константа».
//! - `sources/google_sheets.rs` — источник Google Таблицы + парсеры.
//! - `sources/todoist.rs` — источник Todoist (заглушка).
//! - `connectors.rs` — структуры коннекторов, файловая привязка, пути.
//! - `binding.rs` — структура привязки `ApiBinding` и её статус.
//! - `service.rs` — `SyncService`: состояние, CRUD, жизненный цикл потока.
//! - `sync_loop.rs` — фоновый цикл опроса и применение результатов.

pub mod binding;
pub mod connectors;
pub mod service;
pub mod source;
pub mod sources;
pub mod static_source;
pub mod sync_loop;

pub use binding::{ApiBinding, BindingStatus};
pub use connectors::{ConnectorConfig, ConnectorFile, ConnectorInfo};
pub use service::SyncService;
pub use source::{MetricSource, SourceRegistry, SourceType};

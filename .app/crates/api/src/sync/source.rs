//! Источник данных показателей: трейт, доступные источники и реестр.
//!
//! Единственная ответственность — абстракция над источниками (константа, Google Таблицы, Todoist).

use polygram_core::types::{MetricKind, MetricValue, ParseHint};
use serde_json::Value;

use super::static_source::StaticSource;
use super::sources::google_sheets::GoogleSheetsSource;
use super::sources::todoist::TodoistSource;

/// Трейт для источников данных показателей.
pub trait MetricSource: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn supported_kinds(&self) -> Vec<MetricKind>;
    fn fetch(
        &self,
        cell: &str,
        config: Option<&Value>,
        kind: MetricKind,
        parse_hint: &ParseHint,
    ) -> Result<MetricValue, String>;
    fn placeholder(&self) -> &str;
}

/// Список зарегистрированных источников.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceType {
    Static,
    GoogleSheets,
    Todoist,
}

impl MetricSource for SourceType {
    fn id(&self) -> &str {
        match self {
            SourceType::Static => "static",
            SourceType::GoogleSheets => "google-sheets",
            SourceType::Todoist => "todoist",
        }
    }
    fn name(&self) -> &str {
        match self {
            SourceType::Static => "Константа (тест)",
            SourceType::GoogleSheets => "Google Таблицы",
            SourceType::Todoist => "Todoist",
        }
    }
    fn supported_kinds(&self) -> Vec<MetricKind> {
        match self {
            SourceType::Static => vec![
                MetricKind::Number,
                MetricKind::Text,
                MetricKind::Binary,
            ],
            SourceType::GoogleSheets => {
                vec![
                    MetricKind::Number,
                    MetricKind::Date,
                    MetricKind::Deadline,
                    MetricKind::Text,
                ]
            }
            SourceType::Todoist => {
                vec![MetricKind::Deadline, MetricKind::Duration, MetricKind::Text]
            }
        }
    }
    fn fetch(
        &self,
        cell: &str,
        config: Option<&Value>,
        kind: MetricKind,
        parse_hint: &ParseHint,
    ) -> Result<MetricValue, String> {
        match self {
            SourceType::Static => StaticSource.fetch(cell, config, kind, parse_hint),
            SourceType::GoogleSheets => GoogleSheetsSource.fetch(cell, config, kind, parse_hint),
            SourceType::Todoist => TodoistSource.fetch(cell, config, kind, parse_hint),
        }
    }
    fn placeholder(&self) -> &str {
        match self {
            SourceType::Static => "42",
            SourceType::GoogleSheets => "https://docs.google.com/spreadsheets/d/... или Sheet1!A1",
            SourceType::Todoist => "todoist:task:ID:due_date",
        }
    }
}

/// Реестр источников данных.
#[derive(Clone, Default)]
pub struct SourceRegistry {
    sources: std::collections::HashMap<String, SourceType>,
}

impl SourceRegistry {
    pub fn new() -> Self {
        let mut sources: std::collections::HashMap<String, SourceType> =
            std::collections::HashMap::new();
        sources.insert("static".to_owned(), SourceType::Static);
        sources.insert("google-sheets".to_owned(), SourceType::GoogleSheets);
        sources.insert("todoist".to_owned(), SourceType::Todoist);
        Self { sources }
    }

    pub fn get(&self, id: &str) -> Option<SourceType> {
        self.sources.get(id).copied()
    }
}

//! Источник «Todoist» (заглушка — требует OAuth).

use polygram_core::types::{MetricKind, MetricValue, ParseHint};
use serde_json::Value;

use crate::sync::MetricSource;

/// Источник: Todoist (заглушка — требует OAuth).
pub struct TodoistSource;

impl MetricSource for TodoistSource {
    fn id(&self) -> &str {
        "todoist"
    }
    fn name(&self) -> &str {
        "Todoist"
    }
    fn supported_kinds(&self) -> Vec<MetricKind> {
        vec![MetricKind::Deadline, MetricKind::Duration, MetricKind::Text]
    }
    fn fetch(
        &self,
        cell: &str,
        _config: Option<&Value>,
        kind: MetricKind,
        _parse_hint: &ParseHint,
    ) -> Result<MetricValue, String> {
        if cell.is_empty() {
            return Err(
                "укажите ссылку на задачу Todoist (например todoist:task:123:due_date)".to_owned(),
            );
        }
        let hash: u32 = cell
            .bytes()
            .fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
        match kind {
            MetricKind::Deadline => {
                let days_from_now = (hash % 30) as i64;
                let dt = chrono::Utc::now() + chrono::Duration::days(days_from_now);
                Ok(MetricValue::Deadline(dt))
            }
            MetricKind::Duration => {
                let minutes = (hash % 120) as i64;
                Ok(MetricValue::Duration(minutes * 60_000))
            }
            MetricKind::Text => Ok(MetricValue::Text(format!("todoist:{cell}"))),
            _ => Err(format!("Todoist не поддерживает тип {:?}", kind)),
        }
    }
    fn placeholder(&self) -> &str {
        "todoist:task:ID:due_date"
    }
}

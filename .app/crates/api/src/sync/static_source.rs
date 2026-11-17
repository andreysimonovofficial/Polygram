//! Источник «Константа (тест)».

use polygram_core::types::{MetricKind, MetricValue, ParseHint};
use serde_json::Value;

use super::MetricSource;

/// Источник: constantes.
pub struct StaticSource;

impl MetricSource for StaticSource {
    fn id(&self) -> &str {
        "static"
    }
    fn name(&self) -> &str {
        "Константа (тест)"
    }
    fn supported_kinds(&self) -> Vec<MetricKind> {
        vec![
            MetricKind::Number,
            MetricKind::Text,
            MetricKind::Binary,
        ]
    }
    fn fetch(
        &self,
        cell: &str,
        _config: Option<&Value>,
        kind: MetricKind,
        _parse_hint: &ParseHint,
    ) -> Result<MetricValue, String> {
        match kind {
            MetricKind::Number => cell
                .parse::<f64>()
                .map(MetricValue::Number)
                .map_err(|_| format!("не удалось разобрать константу: {cell}")),
            MetricKind::Text => Ok(MetricValue::Text(cell.to_owned())),
            MetricKind::Binary => match cell.trim().to_lowercase().as_str() {
                "true" | "1" | "да" | "yes" => Ok(MetricValue::Binary(true)),
                "false" | "0" | "нет" | "no" => Ok(MetricValue::Binary(false)),
                _ => Err(format!("не удалось разобрать булево значение: {cell}")),
            },
            _ => Err(format!(
                "статический источник не поддерживает тип {:?}",
                kind
            )),
        }
    }
    fn placeholder(&self) -> &str {
        "42"
    }
}

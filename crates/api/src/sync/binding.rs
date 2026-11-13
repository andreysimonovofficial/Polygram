//! Структура привязки показателя к внешнему источнику и её статус.

use serde::{Deserialize, Serialize};

/// Привязка показателя объекта к внешнему источнику.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiBinding {
    pub id: String,
    pub object_id: String,
    pub entry_id: String,
    pub connector: String,
    pub cell: String,
    #[serde(default = "default_interval")]
    pub interval_secs: u64,
}

fn default_interval() -> u64 {
    30
}

/// Статус одной привязки.
#[derive(Clone, Debug, Default, Serialize)]
pub struct BindingStatus {
    pub ok: bool,
    pub last_value: Option<String>,
    pub last_updated: Option<String>,
    pub error: Option<String>,
}

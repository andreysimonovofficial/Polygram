//! «Терминал»: единый язык команд.
//! Команды сериализуются в JSON (поле `cmd`), поэтому их можно подавать из файла или по сети.
//!
//! Показатель (MetricDef) — глобальная сущность. Запись контейнера (ObjectMetric) — локальная в объекте.
//! Связь соединяет запись Выхода с записью Запроса разных объектов.

use crate::types::*;
use serde::{Deserialize, Serialize};

fn null_value() -> MetricValue {
    MetricValue::Null
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Command {
    AddObject {
        #[serde(default)]
        id: Option<String>,
        name: String,
        #[serde(default)]
        kind: NodeKind,
        /// Автор создания объекта для ленты чата (user / claude / qwen / ...).
        #[serde(default)]
        author: Option<String>,
        /// Клетка левого верхнего угла. Если не задано — ставится автоматически.
        #[serde(default)]
        at: Option<[i32; 2]>,
        /// Внешний id родительского отдела (None — корень).
        #[serde(default)]
        parent: Option<String>,
    },
    RemoveObject {
        object: String,
    },
    RenameObject {
        object: String,
        name: String,
    },
    MoveObject {
        object: String,
        at: [i32; 2],
    },
    /// Изменить родительский отдел объекта (None — корень).
    /// Запрещено назначать родителем самого себя или своего потомка (цикл).
    SetObjectParent {
        object: String,
        #[serde(default)]
        parent: Option<String>,
    },

    /// Создать глобальный показатель (единый для всего холста).
    AddMetricDef {
        #[serde(default)]
        id: Option<String>,
        name: String,
        #[serde(default)]
        kind: MetricKind,
        #[serde(default)]
        unit: String,
        #[serde(default)]
        api: Option<MetricApi>,
    },
    RemoveMetricDef {
        metric_def: String,
    },
    RenameMetricDef {
        metric_def: String,
        name: String,
    },
    SetMetricDefUnit {
        metric_def: String,
        unit: String,
    },

    /// Добавить запись показателя в контейнер объекта.
    AddMetricEntry {
        object: String,
        dir: Direction,
        #[serde(default)]
        id: Option<String>,
        metric_def: String,
        name: String,
        #[serde(default = "null_value")]
        value: MetricValue,
        #[serde(default = "null_value")]
        required: MetricValue,
        #[serde(default)]
        max: Option<MetricValue>,
        #[serde(default = "null_value")]
        fact: MetricValue,
    },
    RemoveMetricEntry {
        object: String,
        entry: String,
    },
    SetValue {
        object: String,
        entry: String,
        value: MetricValue,
    },
    SetRequired {
        object: String,
        entry: String,
        required: MetricValue,
    },
    SetMax {
        object: String,
        entry: String,
        max: Option<MetricValue>,
    },
    SetFact {
        object: String,
        entry: String,
        fact: MetricValue,
    },

    /// Связь: запись Выхода источника → запись Запроса цели.
    /// throughput — доля Выхода в процентах (0..=100) или абсолютное значение.
    Connect {
        from_object: String,
        from_entry: String,
        to_object: String,
        to_entry: String,
        #[serde(default = "default_throughput")]
        throughput: f64,
    },
    Disconnect {
        from_object: String,
        from_entry: String,
        to_object: String,
        to_entry: String,
    },

    AutoLayout,
    RerouteAll,

    /// Управление сервером — единые для GUI, MCP и HTTP.
    RestartServer,
    ChangePort { port: u16 },
    Shutdown,
    ReloadConfig,
}

fn default_throughput() -> f64 {
    100.0
}

/// Результат успешной команды.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Applied {
    /// Ревизия после выполнения.
    pub rev: u64,
    /// Идентификатор созданного объекта, показателя или записи.
    pub created: Option<String>,
    /// Клетка, куда объект реально встал.
    pub at: Option<[i32; 2]>,
}

pub fn default_unit(kind: MetricKind) -> &'static str {
    match kind {
        MetricKind::Number => "ед.",
        MetricKind::Time => "чч:мм",
        MetricKind::Date => "дд.мм.гггг",
        MetricKind::Deadline => "дд.мм.гггг чч:мм",
        MetricKind::Percentage => "%",
        MetricKind::Binary => "",
        MetricKind::Duration => "чч:мм",
        MetricKind::Money => "₽",
        MetricKind::Text => "",
        MetricKind::Formula => "",
        MetricKind::Timestamp => "",
    }
}

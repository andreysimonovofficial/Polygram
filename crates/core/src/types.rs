//! Базовые типы ядра: индексы, виды показателей, статусы.
//!
//! Показатель (MetricDef) — глобальная сущность на весь холст.
//! Запись контейнера (ObjectMetric) — локальная в объекте, ссылается на MetricDef,
//! хранит своё значение, необходимое и максимум для этого объекта.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Допуск сравнения чисел (перенесён из MVP).
pub const EPS: f64 = 1e-4;

/// Максимум записей с одной стороны контейнера объекта (запросы или выходы).
pub const MAX_METRICS_PER_SIDE: usize = 8;
/// Лимит показателей, которое можно добавить через интерфейс
/// на одну сторону (Запрос / Выход) одного объекта.
pub const MAX_UI_METRICS_PER_SIDE: usize = 4;

/// Служебное значение «нет» для связных списков.
pub(crate) const NONE: u32 = u32::MAX;

macro_rules! ix_type {
    ($name:ident) => {
        #[derive(
            Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
        )]
        pub struct $name(pub u32);
        impl $name {
            #[inline]
            pub fn idx(self) -> usize {
                self.0 as usize
            }
        }
    };
}
ix_type!(ObjectIx);
ix_type!(MetricDefIx);
ix_type!(EntryIx);
ix_type!(EdgeIx);

/// Логический тип показателя. Определяет семантику значения и способ расчёта статуса.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum MetricKind {
    #[default]
    Number,      // число (шт., кол-во, рейтинг)
    Time,        // обратная совместимость: хранится как Number
    Date,        // абсолютная дата/время (точка на таймлайне)
    Deadline,    // срок окончания — статус считается относительно now (просрочено/в срок)
    Percentage,  // 0..=100
    Binary,      // да/нет, выполнено/нет
    Duration,    // обратная совместимость: интервал, хранится как Number (мс)
    Money,       // сумма в минорных единицах (копейки/центы), без float-ошибок
    Text,        // свободная строка
    Formula,     // вычисляемое
    Timestamp,   // метка времени создания/обновления (read-only из системы)
}

/// Типизированное значение показателя. Единое хранилище для всех источников.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
#[non_exhaustive]
pub enum MetricValue {
    Null,
    Number(f64),
    Date(DateTime<Utc>),
    Deadline(DateTime<Utc>),
    Duration(i64),    // миллисекунды
    Percentage(f64),  // 0..=100
    Binary(bool),
    Money(i64),       // минорные единицы (копейки)
    Text(String),
}

impl Default for MetricValue {
    fn default() -> Self {
        MetricValue::Null
    }
}

impl MetricValue {
    /// Извлечь f64 для числовых операций (Number, Percentage, Money, Duration).
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            MetricValue::Number(v) => Some(*v),
            MetricValue::Percentage(v) => Some(*v),
            MetricValue::Money(v) => Some(*v as f64),
            MetricValue::Duration(v) => Some(*v as f64),
            _ => None,
        }
    }

    /// Извлечь DateTime для временных операций (Date, Deadline).
    pub fn as_datetime(&self) -> Option<DateTime<Utc>> {
        match self {
            MetricValue::Date(v) => Some(*v),
            MetricValue::Deadline(v) => Some(*v),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, MetricValue::Null)
    }
}

impl std::fmt::Display for MetricValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricValue::Null => write!(f, "–"),
            MetricValue::Number(v) => write!(f, "{v}"),
            MetricValue::Date(dt) => write!(f, "{}", dt.format("%d.%m.%Y")),
            MetricValue::Deadline(dt) => write!(f, "{}", dt.format("%d.%m.%Y %H:%M")),
            MetricValue::Duration(ms) => {
                let secs = *ms / 1000;
                let h = secs / 3600;
                let m = (secs % 3600) / 60;
                write!(f, "{h}:{m:02}")
            }
            MetricValue::Percentage(v) => write!(f, "{v}%"),
            MetricValue::Binary(b) => write!(f, "{}", if *b { "да" } else { "нет" }),
            MetricValue::Money(v) => write!(f, "{v}"),
            MetricValue::Text(s) => write!(f, "{s}"),
        }
    }
}

/// Направление контейнера: Запрос (вход) или Выход.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    #[default]
    Request,
    Output,
}

/// Вид объекта: Процесс или Отдел.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum NodeKind {
    #[default]
    Process,
    Department,
}

impl NodeKind {
    pub fn label_ru(self) -> &'static str {
        match self {
            Self::Process => "Процесс",
            Self::Department => "Отдел",
        }
    }
}

/// Область видимости команды. Сейчас только корень; позже — отделы (субагенты).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Scope {
    #[default]
    Root,
}

/// Глобальный показатель: единый для всего холста. Значения в объектах могут отличаться.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetricDef {
    pub id: String,
    pub name: String,
    pub kind: MetricKind,
    pub unit: String,
    pub api: Option<MetricApi>,
}

/// Запись контейнера объекта: ссылка на глобальный показатель с локальными значениями.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ObjectMetric {
    pub id: String,
    pub metric_def: MetricDefIx,
    pub direction: Direction,
    pub value: MetricValue,
    pub required: MetricValue,
    pub max: Option<MetricValue>,
    pub fact: MetricValue,
    pub alive: bool,
}

/// Единая «правда» о состоянии показателя (8 кодов, как в MVP/MCP).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusCode {
    Ok,
    Warning,
    Deficit,
    Surplus,
    Unconnected,
    Invalid,
    Disabled,
    Unknown,
}

impl StatusCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Warning => "warning",
            Self::Deficit => "deficit",
            Self::Surplus => "surplus",
            Self::Unconnected => "unconnected",
            Self::Invalid => "invalid",
            Self::Disabled => "disabled",
            Self::Unknown => "unknown",
        }
    }

    pub fn label_ru(self) -> &'static str {
        match self {
            Self::Ok => "Норма",
            Self::Warning => "Предупреждение",
            Self::Deficit => "Дефицит",
            Self::Surplus => "Избыток",
            Self::Unconnected => "Не подключено",
            Self::Invalid => "Ошибка",
            Self::Disabled => "Отключено",
            Self::Unknown => "Неизвестно",
        }
    }
}

/// Подсказка парсинга значения из источника (для Date/Deadline/Number и т.д.).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ParseHint {
    #[default]
    None,
    DateFormat(String),     // для Date/Deadline: "dd.MM.yyyy", "ISO 8601", …
    DurationUnit(String),   // для Duration: "days", "hours", "iso8601"
    NumberFormat(String),   // для Number/Money: "0.00", "ru", "en"
}

/// Привязка показателя к внешнему источнику API.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetricApi {
    pub connector: String,   // "google-sheets" | "todoist" | "static" | "jira" | …
    pub cell: String,        // "Sheet1!B5" | "todoist:task:123:due_date" | "42"
    #[serde(default)]
    pub parse_hint: ParseHint,
}

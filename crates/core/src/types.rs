//! Базовые типы ядра: индексы, виды показателей, статусы.
//!
//! Показатель (MetricDef) — глобальная сущность на весь холст.
//! Запись контейнера (ObjectMetric) — локальная в объекте, ссылается на MetricDef,
//! хранит своё значение, необходимое и максимум для этого объекта.

use serde::{Deserialize, Serialize};

/// Допуск сравнения чисел (перенесён из MVP).
pub const EPS: f64 = 1e-4;

/// Максимум записей с одной стороны контейнера объекта (запросы или выходы).
pub const MAX_METRICS_PER_SIDE: usize = 8;

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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricKind {
    #[default]
    Number,
    Time,
    Percentage,
    Binary,
}

/// Направление контейнера: Запрос (вход) или Выход.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    #[default]
    Request,
    Output,
}

/// Вид объекта: Процесс, Отдел или Портал.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum NodeKind {
    #[default]
    Process,
    Department,
    Portal,
}

impl NodeKind {
    pub fn label_ru(self) -> &'static str {
        match self {
            Self::Process => "Процесс",
            Self::Department => "Отдел",
            Self::Portal => "Портал",
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
    pub value: f64,
    pub required: f64,
    pub max: Option<f64>,
    pub fact: f64,
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

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetricApi {
    pub connector: String,
    pub cell: String,
}

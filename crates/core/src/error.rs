//! Ошибки ядра.

use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum EngineError {
    ObjectNotFound(String),
    MetricNotFound { object: String, metric: String },
    MetricDefNotFound(String),
    EdgeNotFound,
    DuplicateId(String),
    CellOccupied { gx: i32, gy: i32 },
    NoRoom,
    TooManyMetrics { object: String, max: usize },
    InvalidNumber(&'static str),
    WrongDirection(&'static str),
    InvalidConnection(String),
    EdgeExists,
    UnsupportedFormat(String),
    Document(String),
    Stopped,
}

impl EngineError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::ObjectNotFound(_) => "object_not_found",
            Self::MetricNotFound { .. } => "metric_not_found",
            Self::MetricDefNotFound(_) => "metric_def_not_found",
            Self::EdgeNotFound => "edge_not_found",
            Self::DuplicateId(_) => "duplicate_id",
            Self::CellOccupied { .. } => "cell_occupied",
            Self::NoRoom => "no_room",
            Self::TooManyMetrics { .. } => "too_many_metrics",
            Self::InvalidNumber(_) => "invalid_number",
            Self::WrongDirection(_) => "wrong_direction",
            Self::InvalidConnection(_) => "invalid_connection",
            Self::EdgeExists => "edge_exists",
            Self::UnsupportedFormat(_) => "unsupported_format",
            Self::Document(_) => "invalid_document",
            Self::Stopped => "engine_stopped",
        }
    }
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ObjectNotFound(id) => write!(f, "Объект «{id}» не найден"),
            Self::MetricNotFound { object, metric } => {
                write!(f, "Запись «{metric}» у объекта «{object}» не найдена")
            }
            Self::MetricDefNotFound(id) => write!(f, "Показатель «{id}» не найден"),
            Self::EdgeNotFound => write!(f, "Связь не найдена"),
            Self::DuplicateId(id) => write!(f, "Идентификатор «{id}» уже занят"),
            Self::CellOccupied { gx, gy } => {
                write!(f, "Клетки ({gx}, {gy}) заняты другим объектом")
            }
            Self::NoRoom => write!(f, "Не нашлось свободного места на сетке"),
            Self::TooManyMetrics { object, max } => {
                write!(f, "У объекта «{object}» не может быть больше {max} записей с одной стороны")
            }
            Self::InvalidNumber(what) => write!(f, "Некорректное число: {what}"),
            Self::WrongDirection(what) => write!(f, "Неверное направление записи: {what}"),
            Self::InvalidConnection(why) => write!(f, "Недопустимая связь: {why}"),
            Self::EdgeExists => write!(f, "Такая связь уже есть"),
            Self::UnsupportedFormat(found) => write!(
                f,
                "Файл создан другой версией формата ({found}). Эта версия открывает только формат polygram/2"
            ),
            Self::Document(why) => write!(f, "Файл проекта повреждён: {why}"),
            Self::Stopped => write!(f, "Движок остановлен"),
        }
    }
}

impl std::error::Error for EngineError {}

pub type Result<T> = std::result::Result<T, EngineError>;

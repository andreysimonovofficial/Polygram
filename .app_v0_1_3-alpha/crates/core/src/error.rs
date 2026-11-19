//! Ошибки движка.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("объект не найден: {0}")]
    ObjectNotFound(String),
    #[error("показатель не найден: {0}")]
    MetricDefNotFound(String),
    #[error("запись не найдена: {0}")]
    EntryNotFound(String),
    #[error("стоп")]
    Stopped,
    #[error("неверное направление: {0}")]
    WrongDirection(String),
    #[error("некорректное соединение: {0}")]
    InvalidConnection(String),
    #[error("связь уже существует")]
    EdgeExists,
    #[error("связь не найдена")]
    EdgeNotFound,
    #[error("ячейка занята: gx={gx}, gy={gy}")]
    CellOccupied { gx: i32, gy: i32 },
}

pub type Result<T> = std::result::Result<T, EngineError>;

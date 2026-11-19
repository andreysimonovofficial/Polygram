//! Геометрия объектов и записей контейнеров для холста.

use crate::layout::Rect;
use crate::types::{MetricValue, NodeKind};

/// Редко меняющаяся часть для холста: положение карточек и записи контейнеров.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EntryGeom {
    pub ix: u32,
    pub name: String,
    pub value: MetricValue,
    pub supplied: MetricValue,
    pub unit: String,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ObjectGeom {
    pub ix: u32,
    pub id: String,
    pub name: String,
    pub kind: NodeKind,
    pub rect: Rect,
    pub requests: Vec<EntryGeom>,
    pub results: Vec<EntryGeom>,
    pub parent: Option<String>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EdgeGeom {
    pub ix: u32,
    pub from: u32,
    pub to: u32,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Geometry {
    pub rev: u64,
    pub objects: Vec<ObjectGeom>,
    pub edges: Vec<EdgeGeom>,
}

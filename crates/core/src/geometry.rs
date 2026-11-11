use crate::layout::Rect;
use crate::types::{MetricValue, NodeKind};

/// Редко меняющаяся часть для холста: положение карточек и линии.
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
    pub outputs: Vec<EntryGeom>,
    /// Идентификатор родительского отдела, если объект вложен в отдел.
    pub parent: Option<String>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EdgeGeom {
    pub ix: u32,
    pub from: u32,
    pub to: u32,
    pub points: Vec<crate::layout::Point>,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Geometry {
    pub rev: u64,
    pub objects: Vec<ObjectGeom>,
    pub edges: Vec<EdgeGeom>,
}

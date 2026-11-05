use crate::layout::Rect;
use crate::types::NodeKind;

/// Редко меняющаяся часть для холста: положение карточек и линии.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct EntryGeom {
    pub ix: u32,
    pub name: String,
    pub value: f64,
    pub supplied: f64,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct ObjectGeom {
    pub ix: u32,
    pub id: String,
    pub name: String,
    pub kind: NodeKind,
    pub rect: Rect,
    pub requests: Vec<EntryGeom>,
    pub outputs: Vec<EntryGeom>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct EdgeGeom {
    pub ix: u32,
    pub from: u32,
    pub to: u32,
    pub points: Vec<crate::layout::Point>,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct Geometry {
    pub rev: u64,
    pub objects: Vec<ObjectGeom>,
    pub edges: Vec<EdgeGeom>,
}

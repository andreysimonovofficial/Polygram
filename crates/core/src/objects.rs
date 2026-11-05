use crate::types::*;

/// Данные объекта в плоском массиве.
#[derive(Clone, Debug)]
pub struct ObjectData {
    pub ext_id: String,
    pub name: String,
    pub kind: NodeKind,
    pub parent: Option<ObjectIx>,
    pub requests: Vec<EntryIx>,
    pub outputs: Vec<EntryIx>,
    pub alive: bool,
}

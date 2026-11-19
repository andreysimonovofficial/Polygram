//! Рёбра связей между записями показателей.

use crate::engine::Engine;
use crate::{
    command::Applied,
    error::{EngineError, Result},
    types::*,
};

/// Данные ребра связи между записями показателей.
#[derive(Clone, Debug, PartialEq)]
pub struct EdgeData {
    pub ix: EdgeIx,
    pub from: EntryIx,
    pub to: EntryIx,
    pub throughput: f64,
    pub alive: bool,
}

pub struct EdgeIter<'a> {
    h: &'a crate::Hierarchy,
    cur: usize,
    by_from: bool,
    target: Option<crate::types::EntryIx>,
}

impl<'a> EdgeIter<'a> {
    pub(crate) fn new(
        h: &'a crate::Hierarchy,
        first: u32,
        by_from: bool,
        target: Option<crate::types::EntryIx>,
    ) -> Self {
        Self {
            h,
            cur: first as usize,
            by_from,
            target,
        }
    }
}

impl<'a> Iterator for EdgeIter<'a> {
    type Item = crate::types::EdgeIx;
    fn next(&mut self) -> Option<crate::types::EdgeIx> {
        while self.cur < self.h.edges.len() {
            let e = &self.h.edges[self.cur];
            self.cur += 1;
            if !e.alive {
                continue;
            }
            let matches = if self.by_from {
                e.from == self.target.unwrap_or(e.from)
            } else {
                e.to == self.target.unwrap_or(e.to)
            };
            if matches {
                return Some(e.ix);
            }
        }
        None
    }
}

pub fn handle_connect(
    engine: &mut Engine,
    from_object: &str,
    from_entry: &str,
    to_object: &str,
    to_entry: &str,
    throughput: f64,
) -> Result<Applied> {
    let (_, from) = engine.find_entry(from_object, from_entry)?;
    let (_, to) = engine.find_entry(to_object, to_entry)?;
    let _edge = engine.hier.connect(from, to, throughput)?;
    engine.sync_sizes();
    engine.vals.mark_out(from);
    engine.vals.mark_in(to);
    engine
        .pending
        .push(crate::event::EventKind::EdgeAdded);
    Ok(Applied::default())
}

pub fn handle_disconnect(
    engine: &mut Engine,
    from_object: &str,
    from_entry: &str,
    to_object: &str,
    to_entry: &str,
) -> Result<Applied> {
    let (_, from) = engine.find_entry(from_object, from_entry)?;
    let (_, to) = engine.find_entry(to_object, to_entry)?;
    let edge = engine
        .hier
        .find_edge(from, to)
        .ok_or(EngineError::EdgeNotFound)?;
    engine.hier.remove_edge_internal(edge);
    engine.vals.mark_out(from);
    engine.vals.mark_in(to);
    engine
        .pending
        .push(crate::event::EventKind::EdgeRemoved);
    Ok(Applied::default())
}

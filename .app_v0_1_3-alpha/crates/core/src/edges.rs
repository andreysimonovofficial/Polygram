//! Рёбра связей между записями показателей.

use crate::{
    command::Applied,
    engine::Engine,
    error::Result,
    types::*,
    Hierarchy,
};

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EdgeData {
    pub ix: EdgeIx,
    pub from: EntryIx,
    pub to: EntryIx,
    pub throughput: f64,
    pub alive: bool,
}

pub struct EdgeIter<'a> {
    h: &'a Hierarchy,
    cur: usize,
    by_from: bool,
    target: Option<EntryIx>,
}

impl<'a> EdgeIter<'a> {
    pub(crate) fn new(h: &'a Hierarchy, first: u32, by_from: bool, target: Option<EntryIx>) -> Self {
        Self { h, cur: first as usize, by_from, target }
    }
}

impl<'a> Iterator for EdgeIter<'a> {
    type Item = EdgeIx;
    fn next(&mut self) -> Option<Self::Item> {
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
    _engine: &mut Engine,
    _from_object: &str,
    _from_entry: &str,
    _to_object: &str,
    _to_entry: &str,
    _throughput: f64,
) -> Result<Applied> {
    Ok(Applied::default())
}

pub fn handle_disconnect(
    _engine: &mut Engine,
    _from_object: &str,
    _from_entry: &str,
    _to_object: &str,
    _to_entry: &str,
) -> Result<Applied> {
    Ok(Applied::default())
}

use crate::engine::Engine;
use crate::{
    command::Applied,
    error::{EngineError, Result},
};

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
    let edge = engine.hier.connect(from, to, throughput)?;
    engine.sync_sizes();
    engine.vals.alloc[edge.idx()] = 0.0;
    engine.vals.mark_out(from);
    engine.vals.mark_in(to);
    engine
        .pending
        .push(crate::event::EventKind::EdgeAdded { edge: edge.0 });
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
    engine.remove_edge_internal(edge);
    Ok(Applied::default())
}

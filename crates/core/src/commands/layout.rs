use crate::command::Applied;
use crate::error::Result;
use crate::engine::Engine;
use crate::layout::Rect;

pub fn handle_move_object(
    engine: &mut Engine,
    object: &str,
    at: [i32; 2],
) -> Result<Applied> {
    let ix = engine.find_object(object)?;
    let r = engine.layout.rect_of(ix);
    let new = Rect::new(at[0], at[1], r.w, r.h);
    if new == r {
        return Ok(Applied { at: Some(at), ..Applied::default() });
    }
    if !engine.layout.is_free(&new, Some(ix)) {
        return Err(crate::error::EngineError::CellOccupied { gx: at[0], gy: at[1] });
    }
    engine.layout.replace(ix, new);
    let reqs: Vec<crate::types::EntryIx> = engine.hier.object(ix).requests.clone();
    let outs: Vec<crate::types::EntryIx> = engine.hier.object(ix).outputs.clone();
    for &e in &reqs {
        engine.queue_entry_routes(e);
    }
    for &e in &outs {
        engine.queue_entry_routes(e);
    }
    engine.pending.push(crate::event::EventKind::ObjectMoved { object: ix.0, gx: at[0], gy: at[1] });
    Ok(Applied { at: Some(at), ..Applied::default() })
}

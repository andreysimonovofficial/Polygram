use crate::command::Applied;
use crate::engine::Engine;
use crate::error::Result;

pub fn handle_reroute_all(engine: &mut Engine) -> Result<Applied> {
    engine.layout.reset_lanes();
    let all: Vec<crate::types::EdgeIx> = engine.hier.live_edges().collect();
    for edge in all {
        engine.layout.queue_route(edge);
    }
    Ok(Applied::default())
}

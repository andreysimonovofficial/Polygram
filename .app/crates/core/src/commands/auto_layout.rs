use crate::command::Applied;
use crate::engine::Engine;
use crate::error::Result;

pub fn handle_auto_layout(engine: &mut Engine) -> Result<Applied> {
    engine.layout.auto_layout(&engine.hier);
    engine.pending.push(crate::event::EventKind::LayoutChanged);
    Ok(Applied::default())
}

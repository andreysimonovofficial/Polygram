//! Перемещение объекта.

use crate::engine::Engine;
use crate::{command::Applied, error::Result, layout::Rect};

pub fn handle_move_object(
    engine: &mut Engine,
    object: &str,
    at: [i32; 2],
) -> Result<Applied> {
    let oix = engine.find_object(object)?;
    let rect = Rect::new(at[0], at[1], engine.layout.rect_of(oix).w, engine.layout.rect_of(oix).h);
    engine.layout.replace(oix, rect);
    // Если объект ребёнок отдела — растягиваем отдел под новую позицию.
    if let Some(p) = engine.hier.object(oix).parent {
        engine.expand_department_to_fit(p);
    }
    engine.rev += 1;
    Ok(Applied {
        rev: engine.rev,
        created: None,
        at: Some(at),
    })
}

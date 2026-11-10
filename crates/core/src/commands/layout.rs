use crate::command::Applied;
use crate::engine::Engine;
use crate::error::Result;
use crate::layout::Rect;

pub fn handle_move_object(engine: &mut Engine, object: &str, at: [i32; 2]) -> Result<Applied> {
    let ix = engine.find_object(object)?;
    let r = engine.layout.rect_of(ix);
    let new = Rect::new(at[0], at[1], r.w, r.h);
    if new == r {
        return Ok(Applied {
            at: Some(at),
            ..Applied::default()
        });
    }
    // Проверяем перекрытия, игнорируя самого себя и родительский отдел
    // (ребёнок может находится внутри карточки отдела).
    let parent_ix = engine.hier.object(ix).parent;
    let mut blockers: Vec<u32> = Vec::new();
    engine.layout.objects_overlapping(&new, Some(ix), &mut blockers);
    let truly_blocked = blockers
        .iter()
        .any(|&b| Some(crate::types::ObjectIx(b)) != parent_ix);
    if truly_blocked {
        return Err(crate::error::EngineError::CellOccupied {
            gx: at[0],
            gy: at[1],
        });
    }
    engine.layout.replace(ix, new);
    // Если объект ребёнок отдела — растягиваем отдел под новую позицию.
    if let Some(p) = parent_ix {
        engine.expand_department_to_fit(p);
    }
    let reqs: Vec<crate::types::EntryIx> = engine.hier.object(ix).requests.clone();
    let outs: Vec<crate::types::EntryIx> = engine.hier.object(ix).outputs.clone();
    for &e in &reqs {
        engine.queue_entry_routes(e);
    }
    for &e in &outs {
        engine.queue_entry_routes(e);
    }
    engine.pending.push(crate::event::EventKind::ObjectMoved {
        object: ix.0,
        gx: at[0],
        gy: at[1],
    });
    Ok(Applied {
        at: Some(at),
        ..Applied::default()
    })
}

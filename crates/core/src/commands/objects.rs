use crate::engine::Engine;
use crate::{
    command::Applied,
    error::Result,
    layout::{footprint_for, Rect},
    types::*,
};

pub fn handle_add_object(
    engine: &mut Engine,
    id: Option<String>,
    name: String,
    kind: NodeKind,
    at: Option<[i32; 2]>,
    parent: Option<String>,
    _author: Option<String>,
) -> Result<Applied> {
    let (w, h) = if kind == NodeKind::Department {
        (100, 100)
    } else {
        let (w, h) = footprint_for(0, 0);
        (w as i32, h as i32)
    };
    let parent_ix = match &parent {
        Some(p) => Some(
            engine
                .hier
                .find_object(p)
                .ok_or_else(|| crate::error::EngineError::ObjectNotFound(p.clone()))?,
        ),
        None => None,
    };
    let (ix, gx, gy) = match at {
        Some([x, y]) => {
            let rect = Rect::new(x, y, w, h);
            let placed = if engine.layout.is_free(&rect, parent_ix) {
                (x, y)
            } else if let Some(p) = parent_ix {
                match engine.layout.find_free_in_parent(p, w, h) {
                    Some((nx, ny)) => (nx, ny),
                    None => {
                        return Err(crate::error::EngineError::CellOccupied { gx: x, gy: y });
                    }
                }
            } else {
                return Err(crate::error::EngineError::CellOccupied { gx: x, gy: y });
            };
            let ix = engine.hier.add_object(id, name, kind, parent_ix)?;
            engine.sync_sizes();
            (ix, placed.0, placed.1)
        }
        None => {
            let ix = engine.hier.add_object(id, name, kind, parent_ix)?;
            engine.sync_sizes();
            let (x, y) = if let Some(p) = parent_ix {
                engine
                    .layout
                    .find_free_in_parent(p, w, h)
                    .unwrap_or_else(|| engine.layout.next_auto_slot(w, h))
            } else {
                engine.layout.next_auto_slot(w, h)
            };
            (ix, x, y)
        }
    };
    engine.layout.place(ix, Rect::new(gx, gy, w, h));
    engine
        .pending
        .push(crate::event::EventKind::ObjectAdded { object: ix.0 });
    engine.pending.push(crate::event::EventKind::ObjectMoved {
        object: ix.0,
        gx,
        gy,
    });
    Ok(Applied {
        created: Some(engine.hier.object(ix).ext_id.clone()),
        at: Some([gx, gy]),
        ..Applied::default()
    })
}

pub fn handle_remove_object(engine: &mut Engine, object: &str) -> Result<Applied> {
    let ix = engine.find_object(object)?;
    let edges: Vec<crate::types::EdgeIx> = engine.hier.edges_of_object(ix);
    for edge in edges {
        engine.remove_edge_internal(edge);
    }
    let entries: Vec<crate::types::EntryIx> = {
        let o = engine.hier.object(ix);
        o.requests.iter().chain(o.outputs.iter()).copied().collect()
    };
    for e in entries {
        engine.vals.reset_metric(e);
        engine.hier.remove_object_metric(e);
        engine
            .pending
            .push(crate::event::EventKind::MetricRemoved { metric: e.0 });
    }
    engine.layout.unplace(ix);
    engine.hier.remove_object_slot(ix);
    engine
        .pending
        .push(crate::event::EventKind::ObjectRemoved { object: ix.0 });
    Ok(Applied::default())
}

pub fn handle_rename_object(engine: &mut Engine, object: &str, name: String) -> Result<Applied> {
    let ix = engine.find_object(object)?;
    engine.hier.rename_object(ix, name);
    engine
        .pending
        .push(crate::event::EventKind::ObjectRenamed { object: ix.0 });
    Ok(Applied::default())
}

/// Изменить родительский отдел объекта.
/// Запрещено назначать родителем самого себя или своего потомка.
pub fn handle_set_object_parent(
    engine: &mut Engine,
    object: &str,
    parent: Option<String>,
) -> Result<Applied> {
    let object_ix = engine.find_object(object)?;
    let parent_ix = match &parent {
        Some(p) => Some(
            engine
                .hier
                .find_object(p)
                .ok_or_else(|| crate::error::EngineError::ObjectNotFound(p.clone()))?,
        ),
        None => None,
    };

    // Цикл: объект не может быть своим propres родителем.
    if let Some(p) = parent_ix {
        if p == object_ix {
            return Err(crate::error::EngineError::ObjectNotFound(
                "нельзя назначить родителем самого себя".to_owned(),
            ));
        }
        // Родитель не может быть потомком объекта.
        if engine.hier.is_descendant_of(p, object_ix) {
            return Err(crate::error::EngineError::ObjectNotFound(
                "нельзя назначить родителем своего потомка (цикл)".to_owned(),
            ));
        }
    }

    let old_parent = engine.hier.object(object_ix).parent;
    if old_parent == parent_ix {
        return Ok(Applied::default());
    }

    // Обновляем поле parent в иерархии.
    engine.hier.objects[object_ix.idx()].parent = parent_ix;

    // Если объект перемещён в отдел — растягиваем отдел под него.
    if let Some(p) = parent_ix {
        engine.expand_department_to_fit(p);
    } else if let Some(old_p) = old_parent {
        // Ушли из отдела — попробуем сжать старый отдел.
        engine.shrink_department_to_fit(old_p);
    }

    engine
        .pending
        .push(crate::event::EventKind::ObjectMoved {
            object: object_ix.0,
            gx: engine.layout.rect_of(object_ix).x,
            gy: engine.layout.rect_of(object_ix).y,
        });
    Ok(Applied::default())
}

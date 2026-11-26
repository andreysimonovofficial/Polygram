//! Данные объекта в иерархии.

use crate::types::*;

#[derive(Clone, Debug)]
pub struct ObjectData {
    pub ix: ObjectIx,
    pub id: String,
    pub name: String,
    pub kind: NodeKind,
    pub parent: Option<ObjectIx>,
    pub children: Vec<ObjectIx>,
    pub direction: Option<ContainerDirection>,
    /// Для узла tipo Metric — индекс в `object_metrics` (EntryIx).
    pub entry_ix: Option<EntryIx>,
    pub requests: Vec<EntryIx>,
    pub results: Vec<EntryIx>,
    pub alive: bool,
    /// Имя иконки Lucide для обложки объекта (пусто = иконка по умолчанию для типа).
    pub icon: String,
}

/// Команды объектов: AddObject, RemoveObject, RenameObject, SetObjectParent.

use crate::{
    command::Applied,
    error::Result,
    layout::{footprint_for, Rect},
    Engine,
};

pub fn handle_add_object(
    engine: &mut Engine,
    id: Option<String>,
    name: String,
    kind: NodeKind,
    at: Option<[i32; 2]>,
    parent: Option<String>,
    direction: Option<ContainerDirection>,
) -> Result<Applied> {
    // Контейнер должен иметь направление.
    if kind == NodeKind::Container && direction.is_none() {
        return Err(crate::error::EngineError::InvalidConnection(
            "контейнер должен иметь направление (Запрос или Результат)".to_owned(),
        ));
    }
    // Показатель — лист, не может иметь направления.
    if kind == NodeKind::Metric && direction.is_some() {
        return Err(crate::error::EngineError::InvalidConnection(
            "показатель не может иметь направления".to_owned(),
        ));
    }
    // Родитель должен иметь возможность иметь детей.
    if let Some(p) = &parent {
        let p_ix = engine
            .hier
            .find_object(p)
            .ok_or_else(|| crate::error::EngineError::ObjectNotFound(p.clone()))?;
        if !engine.hier.object(p_ix).kind.can_have_children() {
            return Err(crate::error::EngineError::ObjectNotFound(
                format!(
                    "объект «{}» не может иметь дочерние объекты",
                    engine.hier.object(p_ix).name
                ),
            ));
        }
    }

    let (w, h) = match kind {
        NodeKind::Department => (34, 34),
        NodeKind::Process => {
            let (w, h) = footprint_for(kind, 0, 0);
            (w as i32, h as i32)
        }
        NodeKind::Container => {
            let (w, h) = footprint_for(kind, 0, 0);
            (w as i32, h as i32)
        }
        NodeKind::Metric => {
            let (w, h) = footprint_for(kind, 0, 0);
            (w as i32, h as i32)
        }
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
            let ix = engine.hier.add_object(id, name, kind, parent_ix, direction)?;
            engine.sync_sizes();
            (ix, placed.0, placed.1)
        }
        None => {
            let ix = engine.hier.add_object(id, name, kind, parent_ix, direction)?;
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
    // Если добавляется отдел под родительским отделом — растягиваем родителя под нового ребёнка.
    if kind == NodeKind::Department {
        if let Some(p) = parent_ix {
            engine.expand_department_to_fit(p);
        }
    }
    engine
        .pending
        .push(crate::event::EventKind::ObjectAdded { object: ix.0 });
    engine.pending.push(crate::event::EventKind::ObjectMoved {
        object: ix.0,
        gx,
        gy,
    });
    Ok(Applied {
        created: Some(engine.hier.object(ix).id.clone()),
        at: Some([gx, gy]),
        ..Applied::default()
    })
}

pub fn handle_remove_object(engine: &mut Engine, object: &str) -> Result<Applied> {
    let ix = engine.find_object(object)?;
    let parent_ix = engine.hier.object(ix).parent;
    // Рекурсивно удаляем всех потомков (контейнеры и их метрики).
    let children = engine.hier.object(ix).children.clone();
    for &c in &children {
        let c_kind = engine.hier.object(c).kind;
        if c_kind == NodeKind::Metric {
            let e_ix = engine.hier.object(c).entry_ix;
            if let Some(e) = e_ix {
                let edges: Vec<crate::types::EdgeIx> = match engine.hier.object_metric(e).direction {
                    Direction::Result => engine.hier.out_entries(e).collect(),
                    Direction::Request => engine.hier.in_entries(e).collect(),
                };
                for edge in edges {
                    engine.hier.remove_edge_internal(edge);
                }
                engine.vals.reset_metric(e);
                engine.hier.remove_object_metric(e);
                engine
                    .pending
                    .push(crate::event::EventKind::MetricRemoved { metric: e.0 });
            }
        } else {
            // Рекурсивно удаляем потомков контейнера/отдела/процесса.
            let grand_children = engine.hier.object(c).children.clone();
            for &gc in &grand_children {
                if engine.hier.object(gc).kind == NodeKind::Metric {
                    let e_ix = engine.hier.object(gc).entry_ix;
                    if let Some(e) = e_ix {
                        let edges: Vec<crate::types::EdgeIx> = match engine
                            .hier
                            .object_metric(e)
                            .direction
                        {
                            Direction::Result => engine.hier.out_entries(e).collect(),
                            Direction::Request => engine.hier.in_entries(e).collect(),
                        };
                        for edge in edges {
                            engine.hier.remove_edge_internal(edge);
                        }
                        engine.vals.reset_metric(e);
                        engine.hier.remove_object_metric(e);
                        engine
                            .pending
                            .push(crate::event::EventKind::MetricRemoved { metric: e.0 });
                    }
                }
            }
        }
    }
    let entries: Vec<crate::types::EntryIx> = {
        let o = engine.hier.object(ix);
        o.requests.iter().chain(o.results.iter()).copied().collect()
    };
    for e in entries {
        let edges: Vec<crate::types::EdgeIx> = match engine.hier.object_metric(e).direction {
            Direction::Result => engine.hier.out_entries(e).collect(),
            Direction::Request => engine.hier.in_entries(e).collect(),
        };
        for edge in edges {
            engine.hier.remove_edge_internal(edge);
        }
        engine.vals.reset_metric(e);
        engine.hier.remove_object_metric(e);
        engine
            .pending
            .push(crate::event::EventKind::MetricRemoved { metric: e.0 });
    }
    engine.layout.unplace(ix);
    engine.hier.remove_object_slot(ix);
    // Если удалялся объект из отдела — пытаемся сжать родительский отдел.
    if let Some(p) = parent_ix {
        engine.shrink_department_to_fit(p);
    }
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

    if let Some(p) = parent_ix {
        if p == object_ix {
            return Err(crate::error::EngineError::ObjectNotFound(
                "нельзя назначить родителем самого себя".to_owned(),
            ));
        }
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

    // Обновляем поле parent в ObjectData.
    engine.hier.objects[object_ix.idx()].parent = parent_ix;
    // Переносим объект между списками детей родителей (в ObjectData.children).
    if let Some(old_p) = old_parent {
        engine.hier.objects[old_p.idx()]
            .children
            .retain(|&c| c != object_ix);
    }
    if let Some(p) = parent_ix {
        engine.hier.objects[p.idx()].children.push(object_ix);
    }

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

/// Установить иконку обложки для объекта (Процесс или Отдел).
pub fn handle_set_object_icon(engine: &mut Engine, object: &str, icon: String) -> Result<Applied> {
    let object_ix = engine.find_object(object)?;
    let o = engine.hier.object(object_ix);
    if !matches!(o.kind, NodeKind::Process | NodeKind::Department) {
        return Err(crate::error::EngineError::ObjectNotFound(
            "иконку обложки можно назначить только Процессу или Отделу".to_owned(),
        ));
    }
    engine.hier.objects[object_ix.idx()].icon = icon;
    engine.pending.push(crate::event::EventKind::ObjectMoved {
        object: object_ix.0,
        gx: engine.layout.rect_of(object_ix).x,
        gy: engine.layout.rect_of(object_ix).y,
    });
    Ok(Applied::default())
}

//! Команды записей показателей в контейнерах объектов.

use crate::engine::Engine;
use crate::{command::Applied, error::Result, types::*};

pub fn handle_add_metric_entry(
    engine: &mut Engine,
    object: &str,
    dir: Direction,
    id: Option<String>,
    metric_def: &str,
    name: &str,
    value: MetricValue,
    required: MetricValue,
    max: Option<MetricValue>,
    fact: MetricValue,
) -> Result<Applied> {
    let oix = engine.find_object(object)?;
    // Если цель — Процесс, авто-создаём Контейнер нужного направления.
    let container_ix = if engine.hier.object(oix).kind == NodeKind::Process {
        let want = match dir {
            Direction::Request => ContainerDirection::Request,
            Direction::Result => ContainerDirection::Result,
        };
        let existing = engine
            .hier
            .object(oix)
            .children
            .iter()
            .find(|&&c| {
                let cd = &engine.hier.objects[c.idx()];
                cd.kind == NodeKind::Container && cd.direction == Some(want)
            })
            .copied();
        match existing {
            Some(cix) => cix,
            None => engine.hier.add_object(
                None,
                format!(
                    "{} {}",
                    engine.hier.object(oix).name,
                    want.label_ru()
                ),
                NodeKind::Container,
                Some(oix),
                Some(want),
            )?,
        }
    } else if engine.hier.object(oix).kind != NodeKind::Container {
        return Err(crate::error::EngineError::InvalidConnection(
            "метрику можно добавить только в Контейнер или Процесс".to_owned(),
        ));
    } else {
        oix
    };
    let mdix = engine.find_metric_def(metric_def)?;
    let (n_req, n_out) = {
        let o = engine.hier.object(container_ix);
        (
            o.children.iter().filter(|&&c| {
                let cd = &engine.hier.objects[c.idx()];
                cd.kind == NodeKind::Metric
                    && cd.entry_ix.is_some()
                    && engine.hier.object_metric(cd.entry_ix.unwrap()).direction == Direction::Request
            }).count(),
            o.children.iter().filter(|&&c| {
                let cd = &engine.hier.objects[c.idx()];
                cd.kind == NodeKind::Metric
                    && cd.entry_ix.is_some()
                    && engine.hier.object_metric(cd.entry_ix.unwrap()).direction == Direction::Result
            }).count(),
        )
    };
    let plan = if n_req.max(n_out) <= crate::types::MAX_METRICS_PER_SIDE {
        engine.plan_refit(container_ix, n_req, n_out)?
    } else {
        None
    };
    let e = engine
        .hier
        .add_object_metric(container_ix, dir, id, mdix, name.to_owned())?;
    engine.sync_sizes();
    engine.vals.reset_metric(e);
    engine.vals.value[e.idx()] = value;
    engine.vals.required[e.idx()] = required;
    engine.vals.max[e.idx()] = max;
    engine.vals.fact[e.idx()] = fact;
    engine.vals.mark_metric(&engine.hier, e);
    engine
        .pending
        .push(crate::event::EventKind::MetricAdded { metric: e.0 });
    engine.apply_refit(container_ix, plan);
    Ok(Applied {
        created: Some(engine.hier.object_metric(e).ext_id.clone()),
        ..Applied::default()
    })
}

pub fn handle_remove_metric_entry(
    engine: &mut Engine,
    object: &str,
    entry: &str,
) -> Result<Applied> {
    let (oix, e) = engine.find_entry(object, entry)?;
    let edges: Vec<crate::types::EdgeIx> = match engine.hier.object_metric(e).direction {
        Direction::Result => engine.hier.out_entries(e).collect(),
        Direction::Request => engine.hier.in_entries(e).collect(),
    };
    for edge in edges {
        // edge removal handled by hierarchy
        let _ = edge;
    }
    engine.vals.reset_metric(e);
    engine.hier.remove_object_metric(e);
    engine
        .pending
        .push(crate::event::EventKind::MetricRemoved { metric: e.0 });
    let (n_req, n_out) = {
        let o = engine.hier.object(oix);
        (
            o.children.iter().filter(|&&c| {
                let cd = &engine.hier.objects[c.idx()];
                cd.kind == NodeKind::Metric
                    && cd.entry_ix.is_some()
                    && engine.hier.object_metric(cd.entry_ix.unwrap()).direction == Direction::Request
            }).count(),
            o.children.iter().filter(|&&c| {
                let cd = &engine.hier.objects[c.idx()];
                cd.kind == NodeKind::Metric
                    && cd.entry_ix.is_some()
                    && engine.hier.object_metric(cd.entry_ix.unwrap()).direction == Direction::Result
            }).count(),
        )
    };
    let plan = engine.plan_refit(oix, n_req, n_out).unwrap_or(None);
    engine.apply_refit(oix, plan);
    Ok(Applied::default())
}

pub fn handle_set_value(
    engine: &mut Engine,
    object: &str,
    entry: &str,
    value: MetricValue,
) -> Result<Applied> {
    let (_, e) = engine.find_entry(object, entry)?;
    if crate::engine::same_val(&engine.vals.value[e.idx()], &value) {
        return Ok(Applied::default());
    }
    engine.vals.value[e.idx()] = value;
    engine.vals.mark_out(e);
    for edge in engine.hier.out_entries(e) {
        let to = engine.hier.edge(edge).to;
        engine.vals.mark_in(to);
    }
    engine
        .pending
        .push(crate::event::EventKind::MetricChanged { metric: e.0 });
    Ok(Applied::default())
}

pub fn handle_set_required(
    engine: &mut Engine,
    object: &str,
    entry: &str,
    required: MetricValue,
) -> Result<Applied> {
    let (_, e) = engine.find_entry(object, entry)?;
    if engine.hier.object_metric(e).direction != Direction::Request {
        return Err(crate::error::EngineError::WrongDirection(
            "требование задаётся только у записи Запроса".to_owned(),
        ));
    }
    if crate::engine::same_val(&engine.vals.required[e.idx()], &required) {
        return Ok(Applied::default());
    }
    engine.vals.required[e.idx()] = required;
    engine.vals.mark_in(e);
    engine.vals.mark_sources_of(&engine.hier, e);
    engine
        .pending
        .push(crate::event::EventKind::MetricChanged { metric: e.0 });
    Ok(Applied::default())
}

pub fn handle_set_max(
    engine: &mut Engine,
    object: &str,
    entry: &str,
    max: Option<MetricValue>,
) -> Result<Applied> {
    let (_, e) = engine.find_entry(object, entry)?;
    let new = max;
    if crate::engine::same_val_opt(&engine.vals.max[e.idx()], &new) {
        return Ok(Applied::default());
    }
    engine.vals.max[e.idx()] = new;
    engine.vals.mark_metric(&engine.hier, e);
    engine
        .pending
        .push(crate::event::EventKind::MetricChanged { metric: e.0 });
    Ok(Applied::default())
}

pub fn handle_set_fact(
    engine: &mut Engine,
    object: &str,
    entry: &str,
    fact: MetricValue,
) -> Result<Applied> {
    let (_, e) = engine.find_entry(object, entry)?;
    if crate::engine::same_val(&engine.vals.fact[e.idx()], &fact) {
        return Ok(Applied::default());
    }
    engine.vals.fact[e.idx()] = fact;
    engine
        .pending
        .push(crate::event::EventKind::MetricChanged { metric: e.0 });
    Ok(Applied::default())
}

pub fn handle_reorder_metric_entry(
    engine: &mut Engine,
    object: &str,
    dir: Direction,
    from_ix: usize,
    to_ix: usize,
) -> Result<Applied> {
    let oix = engine.find_object(object)?;
    let container_ix = if engine.hier.object(oix).kind == NodeKind::Process {
        let want = match dir {
            Direction::Request => ContainerDirection::Request,
            Direction::Result => ContainerDirection::Result,
        };
        engine
            .hier
            .object(oix)
            .children
            .iter()
            .find(|&&c| {
                let cd = &engine.hier.objects[c.idx()];
                cd.kind == NodeKind::Container && cd.direction == Some(want)
            })
            .copied()
            .ok_or_else(|| {
                crate::error::EngineError::InvalidConnection(format!(
                    "контейнер {} не найден в процессе",
                    want.label_ru()
                ))
            })?
    } else {
        oix
    };
    engine.hier.reorder_object_metric(container_ix, dir, from_ix, to_ix)?;
    engine.sync_sizes();
    engine
        .pending
        .push(crate::event::EventKind::MetricChanged { metric: 0 });
    Ok(Applied::default())
}

pub fn handle_set_in_process(
    engine: &mut Engine,
    object: &str,
    entry: &str,
    in_process: bool,
) -> Result<Applied> {
    let (_, e) = engine.find_entry(object, entry)?;
    let om = &engine.hier.object_metric(e);
    if om.in_process == in_process {
        return Ok(Applied::default());
    }
    let dir = om.direction;
    engine.hier.object_metrics[e.idx()].in_process = in_process;
    engine.vals.mark_metric(&engine.hier, e);
    match dir {
        Direction::Result => {
            for edge in engine.hier.out_entries(e) {
                let to = engine.hier.edge(edge).to;
                engine.vals.mark_in(to);
            }
        }
        Direction::Request => {
            for edge in engine.hier.in_entries(e) {
                let from = engine.hier.edge(edge).from;
                engine.vals.mark_out(from);
            }
        }
    }
    engine
        .pending
        .push(crate::event::EventKind::MetricChanged { metric: e.0 });
    Ok(Applied::default())
}

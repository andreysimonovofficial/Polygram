use crate::{
    command::Applied,
    error::Result,
    types::*,
};
use crate::engine::Engine;

pub fn handle_add_metric_entry(
    engine: &mut Engine,
    object: &str,
    dir: Direction,
    id: Option<String>,
    metric_def: &str,
    value: f64,
    required: f64,
    max: Option<f64>,
    fact: f64,
) -> Result<Applied> {
    let oix = engine.find_object(object)?;
    let mdix = engine.find_metric_def(metric_def)?;
    if !value.is_finite() {
        return Err(crate::error::EngineError::InvalidNumber("value"));
    }
    if !required.is_finite() {
        return Err(crate::error::EngineError::InvalidNumber("required"));
    }
    if max.map_or(false, |m| !m.is_finite()) {
        return Err(crate::error::EngineError::InvalidNumber("max"));
    }
    if !fact.is_finite() {
        return Err(crate::error::EngineError::InvalidNumber("fact"));
    }
    let (n_req, n_out) = {
        let o = engine.hier.object(oix);
        (
            o.requests.len() + usize::from(dir == Direction::Request),
            o.outputs.len() + usize::from(dir == Direction::Output),
        )
    };
    let plan = if n_req.max(n_out) <= crate::types::MAX_METRICS_PER_SIDE {
        engine.plan_refit(oix, n_req, n_out)?
    } else {
        None
    };
    let e = engine.hier.add_object_metric(oix, dir, id, mdix, String::new())?;
    engine.sync_sizes();
    engine.vals.reset_metric(e);
    engine.vals.value[e.idx()] = value;
    engine.vals.required[e.idx()] = required;
    engine.vals.max[e.idx()] = max.unwrap_or(f64::NAN);
    engine.vals.fact[e.idx()] = fact;
    engine.vals.mark_metric(&engine.hier, e);
    engine.pending.push(crate::event::EventKind::MetricAdded { metric: e.0 });
    engine.apply_refit(oix, plan);
    Ok(Applied {
        created: Some(engine.hier.object_metric(e).ext_id.clone()),
        ..Applied::default()
    })
}

pub fn handle_remove_metric_entry(engine: &mut Engine, object: &str, entry: &str) -> Result<Applied> {
    let (oix, e) = engine.find_entry(object, entry)?;
    let edges: Vec<crate::types::EdgeIx> = match engine.hier.object_metric(e).direction {
        Direction::Output => engine.hier.out_entries(e).collect(),
        Direction::Request => engine.hier.in_entries(e).collect(),
    };
    for edge in edges {
        engine.remove_edge_internal(edge);
    }
    engine.vals.reset_metric(e);
    engine.hier.remove_object_metric(e);
    engine.pending.push(crate::event::EventKind::MetricRemoved { metric: e.0 });
    let (n_req, n_out) = {
        let o = engine.hier.object(oix);
        (o.requests.len(), o.outputs.len())
    };
    let plan = engine.plan_refit(oix, n_req, n_out).unwrap_or(None);
    engine.apply_refit(oix, plan);
    engine.queue_entry_routes(e);
    Ok(Applied::default())
}

pub fn handle_set_value(engine: &mut Engine, object: &str, entry: &str, value: f64) -> Result<Applied> {
    let (_, e) = engine.find_entry(object, entry)?;
    if !value.is_finite() {
        return Err(crate::error::EngineError::InvalidNumber("value"));
    }
    if crate::engine::same_num(engine.vals.value[e.idx()], value) {
        return Ok(Applied::default());
    }
    engine.vals.value[e.idx()] = value;
    engine.vals.mark_out(e);
    for edge in engine.hier.out_entries(e) {
        let to = engine.hier.edge(edge).to;
        engine.vals.mark_in(to);
    }
    engine.pending.push(crate::event::EventKind::MetricChanged { metric: e.0 });
    Ok(Applied::default())
}

pub fn handle_set_required(engine: &mut Engine, object: &str, entry: &str, required: f64) -> Result<Applied> {
    let (_, e) = engine.find_entry(object, entry)?;
    if engine.hier.object_metric(e).direction != Direction::Request {
        return Err(crate::error::EngineError::WrongDirection("требование задаётся только у записи Запроса"));
    }
    if !required.is_finite() {
        return Err(crate::error::EngineError::InvalidNumber("required"));
    }
    if crate::engine::same_num(engine.vals.required[e.idx()], required) {
        return Ok(Applied::default());
    }
    engine.vals.required[e.idx()] = required;
    engine.vals.mark_in(e);
    engine.vals.mark_sources_of(&engine.hier, e);
    engine.pending.push(crate::event::EventKind::MetricChanged { metric: e.0 });
    Ok(Applied::default())
}

pub fn handle_set_max(engine: &mut Engine, object: &str, entry: &str, max: Option<f64>) -> Result<Applied> {
    let (_, e) = engine.find_entry(object, entry)?;
    if max.map_or(false, |x| !x.is_finite()) {
        return Err(crate::error::EngineError::InvalidNumber("max"));
    }
    let new = max.unwrap_or(f64::NAN);
    if crate::engine::same_num(engine.vals.max[e.idx()], new) {
        return Ok(Applied::default());
    }
    engine.vals.max[e.idx()] = new;
    engine.vals.mark_metric(&engine.hier, e);
    engine.pending.push(crate::event::EventKind::MetricChanged { metric: e.0 });
    Ok(Applied::default())
}

pub fn handle_set_fact(engine: &mut Engine, object: &str, entry: &str, fact: f64) -> Result<Applied> {
    let (_, e) = engine.find_entry(object, entry)?;
    if !fact.is_finite() {
        return Err(crate::error::EngineError::InvalidNumber("fact"));
    }
    if crate::engine::same_num(engine.vals.fact[e.idx()], fact) {
        return Ok(Applied::default());
    }
    engine.vals.fact[e.idx()] = fact;
    engine.pending.push(crate::event::EventKind::MetricChanged { metric: e.0 });
    Ok(Applied::default())
}

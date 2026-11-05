use crate::{
    command::Applied,
    error::Result,
    types::*,
};
use crate::engine::Engine;

pub fn handle_add_metric_def(
    engine: &mut Engine,
    id: Option<String>,
    name: String,
    kind: MetricKind,
    unit: String,
    api: Option<crate::types::MetricApi>,
) -> Result<Applied> {
    let ix = engine.hier.add_metric_def(id, name, kind, unit, api)?;
    engine.pending.push(crate::event::EventKind::MetricDefAdded { metric_def: ix.0 });
    Ok(Applied {
        created: Some(engine.hier.metric_def(ix).ext_id.clone()),
        ..Applied::default()
    })
}

pub fn handle_remove_metric_def(engine: &mut Engine, metric_def: &str) -> Result<Applied> {
    let ix = engine.find_metric_def(metric_def)?;
    engine.hier.remove_metric_def(ix);
    engine.pending.push(crate::event::EventKind::MetricDefRemoved { metric_def: ix.0 });
    Ok(Applied::default())
}

pub fn handle_rename_metric_def(engine: &mut Engine, metric_def: &str, name: String) -> Result<Applied> {
    let ix = engine.find_metric_def(metric_def)?;
    engine.hier.rename_metric_def(ix, name);
    engine.pending.push(crate::event::EventKind::MetricDefRenamed { metric_def: ix.0 });
    Ok(Applied::default())
}

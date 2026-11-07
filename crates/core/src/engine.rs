//! Движок — единственный владелец графа.
//! Три хранилища: Иерархия (структура) · Значения/статусы (быстро меняющееся) · Архитектура графа (положение и линии).
//! Все изменения идут командами (`Command`); все результаты — события (`Event`) с номером ревизии.

use crate::{
    command::*,
    commit::finish,
    error::{EngineError, Result},
    event::{EventGap, EventKind},
    hierarchy::Hierarchy,
    layout::Layout,
    types::*,
    values::Values,
};

pub(crate) fn same_num(a: f64, b: f64) -> bool {
    (a.is_nan() && b.is_nan()) || a == b
}

#[derive(Clone, Debug)]
pub struct Engine {
    pub(crate) name: String,
    pub(crate) hier: Hierarchy,
    pub(crate) vals: Values,
    pub(crate) layout: Layout,
    pub(crate) log: crate::event::EventLog,
    pub(crate) rev: u64,
    pub(crate) geo_rev: u64,
    pub(crate) pending: Vec<EventKind>,
    pub(crate) scratch_changes: Vec<(crate::types::EntryIx, crate::types::StatusCode, crate::types::StatusCode)>,
}

const EVENT_LOG_CAPACITY: usize = 200_000;

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        let mut engine = Self::with_capacity(0, 0);
        let _ = engine.hier.add_metric_def(
            Some("по умолчанию".to_owned()),
            "по умолчанию".to_owned(),
            MetricKind::Number,
            "ед.".to_owned(),
            None,
        );
        engine
    }

    pub fn with_capacity(objects: usize, edges: usize) -> Self {
        Self {
            name: "Новый граф".to_owned(),
            hier: Hierarchy::with_capacity(objects, edges),
            vals: Values::new(),
            layout: Layout::new(),
            log: crate::event::EventLog::new(EVENT_LOG_CAPACITY),
            rev: 0,
            geo_rev: 0,
            pending: Vec::new(),
            scratch_changes: Vec::new(),
        }
    }

    // ---------- чтение ----------
    pub fn rev(&self) -> u64 {
        self.rev
    }
    pub fn geometry_rev(&self) -> u64 {
        self.geo_rev
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }
    pub fn hierarchy(&self) -> &Hierarchy {
        &self.hier
    }
    pub fn values(&self) -> &Values {
        &self.vals
    }
    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    pub fn events_since(
        &self,
        rev: u64,
    ) -> std::result::Result<Vec<crate::event::Event>, EventGap> {
        self.log.since(rev)
    }

    // ---------- команды ----------
    pub fn apply(&mut self, _scope: Scope, cmd: Command) -> Result<crate::command::Applied> {
        let result = crate::commands::exec(self, cmd);
        finish(self);
        result.map(|mut a| {
            a.rev = self.rev;
            a
        })
    }

    pub fn apply_batch(
        &mut self,
        _scope: Scope,
        cmds: Vec<Command>,
    ) -> std::result::Result<Vec<crate::command::Applied>, (usize, EngineError)> {
        let mut out = Vec::with_capacity(cmds.len());
        for (i, cmd) in cmds.into_iter().enumerate() {
            match crate::commands::exec(self, cmd) {
                Ok(a) => out.push(a),
                Err(e) => {
                    finish(self);
                    return Err((i, e));
                }
            }
        }
        finish(self);
        for a in &mut out {
            a.rev = self.rev;
        }
        Ok(out)
    }
}

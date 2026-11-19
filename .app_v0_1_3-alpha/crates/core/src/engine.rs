//! Движок — единственный владелец графа.

use crate::{
    command::*,
    commit::finish,
    error::{EngineError, Result},
    event::EventKind,
    geometry::{EntryGeom, Geometry, ObjectGeom},
    hierarchy::Hierarchy,
    layout::Layout,
    types::*,
    values::{DynamicState, Values},
    views::MetricDefView,
};

pub(crate) fn same_val(a: &MetricValue, b: &MetricValue) -> bool {
    a == b
}

pub(crate) fn same_val_opt(a: &Option<MetricValue>, b: &Option<MetricValue>) -> bool {
    a == b
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
    pub(crate) scratch_changes: Vec<(EntryIx, StatusCode, StatusCode)>,
    pub(crate) camera_fit_requested: bool,
    pub(crate) auto_screenshot: bool,
    pub(crate) virtual_input: VirtualInputState,
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
            camera_fit_requested: false,
            auto_screenshot: false,
            virtual_input: VirtualInputState::default(),
        }
    }

    pub fn rev(&self) -> u64 {
        self.rev
    }
    pub fn geometry_rev(&self) -> u64 {
        self.geo_rev
    }
    pub fn hierarchy(&self) -> &Hierarchy {
        &self.hier
    }
    pub fn values(&self) -> &Values {
        &self.vals
    }

    pub fn find_object(&self, ext: &str) -> Result<ObjectIx> {
        self.hier.find_object(ext).ok_or_else(|| EngineError::ObjectNotFound(ext.to_owned()))
    }
    pub fn find_metric_def(&self, ext: &str) -> Result<MetricDefIx> {
        self.hier.find_metric_def(ext).ok_or_else(|| EngineError::MetricDefNotFound(ext.to_owned()))
    }
    pub fn find_entry(&self, object: &str, entry: &str) -> Result<(ObjectIx, EntryIx)> {
        let oix = self.find_object(object)?;
        let eix = self
            .hier
            .find_entry(oix, entry)
            .ok_or_else(|| EngineError::EntryNotFound(entry.to_owned()))?;
        Ok((oix, eix))
    }

    pub fn sync_sizes(&mut self) {
        let n_metrics = self.hier.object_metric_count();
        let n_edges = self.hier.edge_count();
        self.vals.ensure(n_metrics, n_edges);
    }

    pub fn plan_refit(&self, _oix: ObjectIx, _n_req: usize, _n_out: usize) -> Result<Option<()>> {
        Ok(None)
    }

    pub fn apply_refit(&mut self, _oix: ObjectIx, _plan: Option<()>) {}

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

    pub fn geometry(&self) -> Geometry {
        self.hier_to_geometry()
    }

    pub fn dynamic_state(&self) -> DynamicState {
        self.vals.snapshot(self.rev)
    }

    pub fn metric_def_views(&self) -> Vec<MetricDefView> {
        self.hier
            .live_metric_defs()
            .map(|ix| {
                let md = self.hier.metric_def(ix);
                MetricDefView {
                    id: md.ext_id.clone(),
                    name: md.name.clone(),
                    kind: md.kind,
                    unit: md.unit.clone(),
                    api: md.api.clone(),
                }
            })
            .collect()
    }

    fn hier_to_geometry(&self) -> Geometry {
        let objects: Vec<ObjectGeom> = self
            .hier
            .live_objects()
            .map(|oix| {
                let o = self.hier.object(oix);
                let requests: Vec<EntryGeom> = o
                    .requests
                    .iter()
                    .filter_map(|&e| {
                        if self.hier.object_metric(e).alive {
                            let meta = self.hier.object_metric(e);
                            let md = self.hier.metric_def(meta.metric_def);
                            Some(EntryGeom {
                                ix: e.0,
                                name: meta.ext_id.clone(),
                                value: self.vals.value[e.idx()].clone(),
                                supplied: self.vals.supplied[e.idx()].clone(),
                                unit: md.unit.clone(),
                            })
                        } else {
                            None
                        }
                    })
                    .collect();
                let results: Vec<EntryGeom> = o
                    .results
                    .iter()
                    .filter_map(|&e| {
                        if self.hier.object_metric(e).alive {
                            let meta = self.hier.object_metric(e);
                            let md = self.hier.metric_def(meta.metric_def);
                            Some(EntryGeom {
                                ix: e.0,
                                name: meta.ext_id.clone(),
                                value: self.vals.value[e.idx()].clone(),
                                supplied: self.vals.supplied[e.idx()].clone(),
                                unit: md.unit.clone(),
                            })
                        } else {
                            None
                        }
                    })
                    .collect();
                let rect = self.layout.rect_of(oix);
                ObjectGeom {
                    ix: oix.0,
                    id: o.id.clone(),
                    name: o.name.clone(),
                    kind: o.kind,
                    rect,
                    requests,
                    results,
                    parent: o.parent.map(|p| self.hier.object(p).id.clone()),
                }
            })
            .collect();
        let edges: Vec<crate::geometry::EdgeGeom> = self
            .hier
            .live_edges()
            .map(|eix| {
                let e = self.hier.edge(eix);
                crate::geometry::EdgeGeom {
                    ix: eix.0,
                    from: e.from.0,
                    to: e.to.0,
                }
            })
            .collect();
        Geometry {
            rev: self.geo_rev,
            objects,
            edges,
        }
    }

    pub fn camera_fit_requested(&self) -> bool {
        self.camera_fit_requested
    }
    pub fn auto_screenshot(&self) -> bool {
        self.auto_screenshot
    }
    pub fn virtual_input(&self) -> &VirtualInputState {
        &self.virtual_input
    }
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct VirtualInputState {
    pub target_x: f32,
    pub target_y: f32,
    pub curve_control_x: f32,
    pub curve_control_y: f32,
    pub mouse_x: f32,
    pub mouse_y: f32,
    pub mouse_left_pressed: bool,
    pub mouse_right_pressed: bool,
    pub mouse_middle_pressed: bool,
    pub keyboard_text: String,
    pub keyboard_keys: Vec<String>,
    pub wheel_delta: (f32, f32),
    pub draw_shape: Option<(u8, f32, f32)>,
    pub drag_target: Option<String>,
    pub drag_to: Option<(f32, f32)>,
    pub path_points: Vec<(f32, f32)>,
    pub path_index: usize,
}

impl VirtualInputState {
    pub fn default() -> Self {
        Self {
            target_x: 0.0,
            target_y: 0.0,
            curve_control_x: 0.0,
            curve_control_y: 0.0,
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_left_pressed: false,
            mouse_right_pressed: false,
            mouse_middle_pressed: false,
            keyboard_text: String::new(),
            keyboard_keys: Vec::new(),
            wheel_delta: (0.0, 0.0),
            draw_shape: None,
            drag_target: None,
            drag_to: None,
            path_points: Vec::new(),
            path_index: 0,
        }
    }
}

//! Мутации иерархии: добавление/удаление объектов, показателей, записей и рёбер.

use crate::{
    edges::EdgeData,
    entries::ObjectMetricData,
    error::{EngineError, Result},
    hierarchy::Hierarchy,
    metric_defs::MetricDefData,
    objects::ObjectData,
    types::*,
};

impl Hierarchy {
    fn fresh_id_internal(&mut self, _prefix: &str) -> String {
        let mut n: u64 = self.id_counter + 1;
        if let Ok(t) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            n = (t.as_millis() as u64 % 999) + 1;
        }
        loop {
            let id = if n <= 999 {
                format!("{:03}", n)
            } else {
                n.to_string()
            };
            if !self.by_ext_object.contains_key(&id) && !self.by_ext_metric_def.contains_key(&id) {
                self.id_counter = n;
                return id;
            }
            n += 1;
        }
    }

    pub fn add_object(
        &mut self,
        ext: Option<String>,
        name: String,
        kind: NodeKind,
        parent: Option<ObjectIx>,
    ) -> Result<ObjectIx> {
        let ext_id = match ext.filter(|s| !s.trim().is_empty()) {
            Some(id) => {
                if self.by_ext_object.contains_key(&id) {
                    return Err(EngineError::DuplicateId(id));
                }
                id
            }
            None => self.fresh_id_internal("o"),
        };
        let data = ObjectData {
            ext_id: ext_id.clone(),
            name,
            kind,
            parent,
            requests: Vec::new(),
            outputs: Vec::new(),
            alive: true,
        };
        let ix = if let Some(slot) = self.free_objects.pop() {
            self.objects[slot as usize] = data;
            ObjectIx(slot)
        } else {
            self.objects.push(data);
            self.children.push(Vec::new());
            ObjectIx((self.objects.len() - 1) as u32)
        };
        if let Some(p) = parent {
            self.children[p.idx()].push(ix);
        }
        self.by_ext_object.insert(ext_id, ix);
        self.live_objects += 1;
        Ok(ix)
    }

    pub fn remove_object_slot(&mut self, ix: ObjectIx) {
        let parent = self.objects[ix.idx()].parent;
        if let Some(p) = parent {
            self.children[p.idx()].retain(|&c| c != ix);
        }
        self.children[ix.idx()].clear();
        let o = &self.objects[ix.idx()];
        debug_assert!(o.requests.is_empty() && o.outputs.is_empty());
        self.by_ext_object.remove(&o.ext_id);
        for &e in &o.requests {
            let em = &self.object_metrics[e.idx()];
            self.by_ext_entry.remove(&(ix, em.ext_id.clone()));
        }
        for &e in &o.outputs {
            let em = &self.object_metrics[e.idx()];
            self.by_ext_entry.remove(&(ix, em.ext_id.clone()));
        }
        let o = &mut self.objects[ix.idx()];
        o.alive = false;
        o.name = String::new();
        o.ext_id = String::new();
        o.requests.clear();
        o.outputs.clear();
        self.free_objects.push(ix.0);
        self.live_objects -= 1;
    }

    pub fn rename_object(&mut self, ix: ObjectIx, name: String) {
        self.objects[ix.idx()].name = name;
    }

    pub fn add_metric_def(
        &mut self,
        ext: Option<String>,
        name: String,
        kind: MetricKind,
        unit: String,
        api: Option<MetricApi>,
    ) -> Result<MetricDefIx> {
        let ext_id = match ext.filter(|s| !s.trim().is_empty()) {
            Some(id) => {
                if self.by_ext_metric_def.contains_key(&id) {
                    return Err(EngineError::DuplicateId(id));
                }
                id
            }
            None => self.fresh_id_internal("m"),
        };
        let data = MetricDefData {
            ext_id: ext_id.clone(),
            name,
            kind,
            unit,
            api,
            alive: true,
        };
        let ix = if let Some(slot) = self.free_metric_defs.pop() {
            self.metric_defs[slot as usize] = data;
            MetricDefIx(slot)
        } else {
            self.metric_defs.push(data);
            MetricDefIx((self.metric_defs.len() - 1) as u32)
        };
        self.by_ext_metric_def.insert(ext_id, ix);
        self.live_metric_defs += 1;
        Ok(ix)
    }

    pub fn remove_metric_def(&mut self, ix: MetricDefIx) {
        let md = &mut self.metric_defs[ix.idx()];
        debug_assert!(md.alive);
        for om in &self.object_metrics {
            if om.alive && om.metric_def == ix {
                panic!(
                    "cannot remove metric_def {} referenced by entry {}",
                    ix.0, om.ext_id
                );
            }
        }
        self.by_ext_metric_def.remove(&md.ext_id);
        md.alive = false;
        md.ext_id = String::new();
        md.name = String::new();
        md.unit = String::new();
        md.api = None;
        self.free_metric_defs.push(ix.0);
        self.live_metric_defs -= 1;
    }

    pub fn rename_metric_def(&mut self, ix: MetricDefIx, name: String) {
        self.metric_defs[ix.idx()].name = name;
    }

    pub fn set_metric_def_unit(&mut self, ix: MetricDefIx, unit: String) {
        self.metric_defs[ix.idx()].unit = unit;
    }

    pub fn add_object_metric(
        &mut self,
        object: ObjectIx,
        dir: Direction,
        ext: Option<String>,
        metric_def: MetricDefIx,
        _name: String,
    ) -> Result<EntryIx> {
        let count = match dir {
            Direction::Request => self.object(object).requests.len(),
            Direction::Output => self.object(object).outputs.len(),
        };
        if count >= MAX_METRICS_PER_SIDE {
            return Err(EngineError::TooManyMetrics {
                object: self.object(object).ext_id.clone(),
                max: MAX_METRICS_PER_SIDE,
            });
        }
        let ext_id = match ext.filter(|s| !s.trim().is_empty()) {
            Some(id) => {
                if self.find_entry(object, &id).is_some() {
                    return Err(EngineError::DuplicateId(id));
                }
                id
            }
            None => {
                let prefix = if dir == Direction::Request { "r" } else { "o" };
                let mut k = count + 1;
                loop {
                    let id = format!("{prefix}{k}");
                    if self.find_entry(object, &id).is_none() {
                        break id;
                    }
                    k += 1;
                }
            }
        };
        let data = ObjectMetricData {
            ext_id: ext_id.clone(),
            object,
            metric_def,
            direction: dir,
            alive: true,
        };
        let ix = if let Some(slot) = self.free_object_metrics.pop() {
            self.object_metrics[slot as usize] = data;
            EntryIx(slot)
        } else {
            self.object_metrics.push(data);
            EntryIx((self.object_metrics.len() - 1) as u32)
        };
        self.first_out.push(NONE);
        self.first_in.push(NONE);
        match dir {
            Direction::Request => self.objects[object.idx()].requests.push(ix),
            Direction::Output => self.objects[object.idx()].outputs.push(ix),
        }
        self.by_ext_entry.insert((object, ext_id), ix);
        self.live_object_metrics += 1;
        Ok(ix)
    }

    pub fn remove_object_metric(&mut self, e: EntryIx) {
        debug_assert!(!self.has_edges(e));
        let (object, ext_id) = {
            let om = &self.object_metrics[e.idx()];
            (om.object, om.ext_id.clone())
        };
        let o = &mut self.objects[object.idx()];
        let list = match self.object_metrics[e.idx()].direction {
            Direction::Request => &mut o.requests,
            Direction::Output => &mut o.outputs,
        };
        list.retain(|&x| x != e);
        let om = &mut self.object_metrics[e.idx()];
        om.alive = false;
        om.ext_id = String::new();
        self.free_object_metrics.push(e.0);
        self.by_ext_entry.remove(&(object, ext_id));
        self.live_object_metrics -= 1;
    }

    pub fn connect(&mut self, from: EntryIx, to: EntryIx, throughput: f64) -> Result<EdgeIx> {
        let (fm, tm) = (
            &self.object_metrics[from.idx()],
            &self.object_metrics[to.idx()],
        );
        if fm.direction != Direction::Output || tm.direction != Direction::Request {
            return Err(EngineError::InvalidConnection(
                "связь идёт только от записи Выхода к записи Запроса".to_owned(),
            ));
        }
        if fm.object == tm.object {
            return Err(EngineError::InvalidConnection(
                "объект нельзя подключить к самому себе".to_owned(),
            ));
        }
        if self.find_edge(from, to).is_some() {
            return Err(EngineError::EdgeExists);
        }
        let data = EdgeData {
            from,
            to,
            next_from: self.first_out[from.idx()],
            next_to: self.first_in[to.idx()],
            throughput,
            alive: true,
        };
        let ix = if let Some(slot) = self.free_edges.pop() {
            self.edges[slot as usize] = data;
            EdgeIx(slot)
        } else {
            self.edges.push(data);
            EdgeIx((self.edges.len() - 1) as u32)
        };
        self.first_out[from.idx()] = ix.0;
        self.first_in[to.idx()] = ix.0;
        self.live_edges += 1;
        Ok(ix)
    }

    pub fn disconnect(&mut self, e: EdgeIx) {
        let (from, to, next_from, next_to) = {
            let d = &self.edges[e.idx()];
            (d.from, d.to, d.next_from, d.next_to)
        };
        if self.first_out[from.idx()] == e.0 {
            self.first_out[from.idx()] = next_from;
        } else {
            let mut cur = self.first_out[from.idx()];
            while cur != NONE {
                let nxt = self.edges[cur as usize].next_from;
                if nxt == e.0 {
                    self.edges[cur as usize].next_from = next_from;
                    break;
                }
                cur = nxt;
            }
        }
        if self.first_in[to.idx()] == e.0 {
            self.first_in[to.idx()] = next_to;
        } else {
            let mut cur = self.first_in[to.idx()];
            while cur != NONE {
                let nxt = self.edges[cur as usize].next_to;
                if nxt == e.0 {
                    self.edges[cur as usize].next_to = next_to;
                    break;
                }
                cur = nxt;
            }
        }
        self.edges[e.idx()].alive = false;
        self.free_edges.push(e.0);
        self.live_edges -= 1;
    }
}

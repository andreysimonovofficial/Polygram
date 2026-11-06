use crate::{
    edges::{EdgeData, EdgeIter},
    entries::ObjectMetricData,
    error::{EngineError, Result},
    metric_defs::MetricDefData,
    objects::ObjectData,
    types::*,
};

/// Иерархия: плоские массивы объектов, показателей-def, записей и ребер.
/// Связи каждого показателя — интрузивные связные списки внутри массива рёбер.
#[derive(Clone, Debug, Default)]
pub struct Hierarchy {
    pub(crate) objects: Vec<ObjectData>,
    pub(crate) metric_defs: Vec<MetricDefData>,
    pub(crate) object_metrics: Vec<ObjectMetricData>,
    pub(crate) edges: Vec<EdgeData>,
    pub(crate) first_out: Vec<u32>,
    pub(crate) first_in: Vec<u32>,
    pub(crate) free_objects: Vec<u32>,
    pub(crate) free_metric_defs: Vec<u32>,
    pub(crate) free_object_metrics: Vec<u32>,
    pub(crate) free_edges: Vec<u32>,
    pub(crate) by_ext_object: std::collections::HashMap<String, ObjectIx>,
    pub(crate) by_ext_metric_def: std::collections::HashMap<String, MetricDefIx>,
    pub(crate) by_ext_entry: std::collections::HashMap<(ObjectIx, String), EntryIx>,
    pub(crate) live_objects: usize,
    pub(crate) live_metric_defs: usize,
    pub(crate) live_object_metrics: usize,
    pub(crate) live_edges: usize,
    pub(crate) id_counter: u64,
}

impl Hierarchy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(objects: usize, edges: usize) -> Self {
        Self {
            objects: Vec::with_capacity(objects),
            metric_defs: Vec::with_capacity(objects),
            object_metrics: Vec::with_capacity(objects * 4),
            edges: Vec::with_capacity(edges),
            first_out: Vec::with_capacity(objects * 4),
            first_in: Vec::with_capacity(objects * 4),
            by_ext_object: std::collections::HashMap::with_capacity(objects),
            by_ext_metric_def: std::collections::HashMap::with_capacity(objects),
            by_ext_entry: std::collections::HashMap::with_capacity(objects * 4),
            ..Self::default()
        }
    }

    // ---------- размеры ----------
    pub fn object_count(&self) -> usize {
        self.live_objects
    }
    pub fn metric_def_count(&self) -> usize {
        self.live_metric_defs
    }
    pub fn object_metric_count(&self) -> usize {
        self.live_object_metrics
    }
    pub fn edge_count(&self) -> usize {
        self.live_edges
    }
    pub fn object_slots(&self) -> usize {
        self.objects.len()
    }
    pub fn metric_def_slots(&self) -> usize {
        self.metric_defs.len()
    }
    pub fn object_metric_slots(&self) -> usize {
        self.object_metrics.len()
    }
    pub fn edge_slots(&self) -> usize {
        self.edges.len()
    }

    // ---------- доступ ----------
    #[inline]
    pub fn object(&self, ix: ObjectIx) -> &ObjectData {
        &self.objects[ix.idx()]
    }
    #[inline]
    pub fn metric_def(&self, ix: MetricDefIx) -> &MetricDefData {
        &self.metric_defs[ix.idx()]
    }
    #[inline]
    pub fn object_metric(&self, ix: EntryIx) -> &ObjectMetricData {
        &self.object_metrics[ix.idx()]
    }
    #[inline]
    pub fn edge(&self, ix: EdgeIx) -> &EdgeData {
        &self.edges[ix.idx()]
    }

    pub fn find_object(&self, ext: &str) -> Option<ObjectIx> {
        self.by_ext_object.get(ext).copied()
    }

    pub fn find_metric_def(&self, ext: &str) -> Option<MetricDefIx> {
        self.by_ext_metric_def.get(ext).copied()
    }

    pub fn find_entry(&self, object: ObjectIx, ext: &str) -> Option<EntryIx> {
        self.by_ext_entry.get(&(object, ext.to_owned())).copied()
    }

    pub fn find_edge(&self, from: EntryIx, to: EntryIx) -> Option<EdgeIx> {
        self.out_entries(from)
            .find(|&e| self.edges[e.idx()].to == to)
    }

    pub fn out_entries(&self, e: EntryIx) -> EdgeIter<'_> {
        EdgeIter {
            h: self,
            cur: self.first_out[e.idx()],
            by_from: true,
        }
    }

    pub fn in_entries(&self, e: EntryIx) -> EdgeIter<'_> {
        EdgeIter {
            h: self,
            cur: self.first_in[e.idx()],
            by_from: false,
        }
    }

    /// Число связей записи.
    pub fn degree(&self, e: EntryIx) -> usize {
        let meta = &self.object_metrics[e.idx()];
        match meta.direction {
            Direction::Output => self.out_entries(e).count(),
            Direction::Request => self.in_entries(e).count(),
        }
    }

    pub fn has_edges(&self, e: EntryIx) -> bool {
        let meta = &self.object_metrics[e.idx()];
        match meta.direction {
            Direction::Output => self.first_out[e.idx()] != NONE,
            Direction::Request => self.first_in[e.idx()] != NONE,
        }
    }

    /// Позиция записи в списке своей стороны контейнера объекта.
    pub fn entry_position(&self, e: EntryIx) -> usize {
        let meta = &self.object_metrics[e.idx()];
        let o = &self.objects[meta.object.idx()];
        let list = match meta.direction {
            Direction::Request => &o.requests,
            Direction::Output => &o.outputs,
        };
        list.iter().position(|&x| x == e).unwrap_or(0)
    }

    pub fn live_objects(&self) -> impl Iterator<Item = ObjectIx> + '_ {
        self.objects
            .iter()
            .enumerate()
            .filter(|(_, o)| o.alive)
            .map(|(i, _)| ObjectIx(i as u32))
    }

    pub fn live_metric_defs(&self) -> impl Iterator<Item = MetricDefIx> + '_ {
        self.metric_defs
            .iter()
            .enumerate()
            .filter(|(_, m)| m.alive)
            .map(|(i, _)| MetricDefIx(i as u32))
    }

    pub fn live_object_metrics(&self) -> impl Iterator<Item = EntryIx> + '_ {
        self.object_metrics
            .iter()
            .enumerate()
            .filter(|(_, m)| m.alive)
            .map(|(i, _)| EntryIx(i as u32))
    }

    pub fn live_edges(&self) -> impl Iterator<Item = EdgeIx> + '_ {
        self.edges
            .iter()
            .enumerate()
            .filter(|(_, e)| e.alive)
            .map(|(i, _)| EdgeIx(i as u32))
    }

    /// Все связи объекта (входящие и исходящие через его записи).
    pub fn edges_of_object(&self, object: ObjectIx) -> Vec<EdgeIx> {
        let o = self.object(object);
        let mut out = Vec::new();
        for &e in &o.outputs {
            out.extend(self.out_entries(e));
        }
        for &e in &o.requests {
            out.extend(self.in_entries(e));
        }
        out
    }

    // ---------- иерархия отделов ----------

    pub fn parent_of(&self, object: ObjectIx) -> Option<ObjectIx> {
        self.object(object).parent
    }

    pub fn children_of(&self, dept: ObjectIx) -> Vec<ObjectIx> {
        self.objects
            .iter()
            .enumerate()
            .filter(|(_, o)| o.alive && o.parent == Some(dept))
            .map(|(i, _)| ObjectIx(i as u32))
            .collect()
    }

    pub fn ancestors_of(&self, object: ObjectIx) -> Vec<ObjectIx> {
        let mut out = Vec::new();
        let mut cur = self.object(object).parent;
        while let Some(p) = cur {
            out.push(p);
            cur = self.object(p).parent;
        }
        out
    }

    pub fn is_descendant_of(&self, object: ObjectIx, dept: ObjectIx) -> bool {
        let mut cur = self.object(object).parent;
        while let Some(p) = cur {
            if p == dept {
                return true;
            }
            cur = self.object(p).parent;
        }
        false
    }

    // ---------- изменение ----------

    fn fresh_id_internal(&mut self, prefix: &str) -> String {
        loop {
            self.id_counter += 1;
            let id = format!("{prefix}{}", self.id_counter);
            if !self.by_ext_object.contains_key(&id) && !self.by_ext_metric_def.contains_key(&id) {
                return id;
            }
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
            ObjectIx((self.objects.len() - 1) as u32)
        };
        self.by_ext_object.insert(ext_id, ix);
        self.live_objects += 1;
        Ok(ix)
    }

    pub fn remove_object_slot(&mut self, ix: ObjectIx) {
        let o = &self.objects[ix.idx()];
        debug_assert!(o.requests.is_empty() && o.outputs.is_empty());
        self.by_ext_object.remove(&o.ext_id);
        // Удаляем все записи этого объекта из индекса
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
        // Проверяем, что нет записей, ссылающихся на этот показатель
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

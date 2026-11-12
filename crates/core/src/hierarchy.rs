use crate::{
    edges::{EdgeData, EdgeIter},
    entries::ObjectMetricData,
    metric_defs::MetricDefData,
    objects::ObjectData,
    types::*,
};

/// Иерархия: плоские массивы объектов, показателей-def, записей и ребер.
/// Связи каждого показателя — интрузивные связные списки внутри массива рёбер.
#[derive(Clone, Debug, Default)]
pub struct Hierarchy {
    pub(crate) objects: Vec<ObjectData>,
    /// Дети каждого отдела, по индексу объекта. Поддерживается синхронно с `objects[*].parent`.
    pub(crate) children: Vec<Vec<ObjectIx>>,
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
            children: Vec::with_capacity(objects),
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
        self.children[dept.idx()].clone()
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
}

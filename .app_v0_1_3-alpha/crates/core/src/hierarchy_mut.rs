//! Мутации иерархии: добавление/удаление объектов, показателей, записей, связей.

use crate::{
    error::Result,
    types::*,
    Hierarchy, MetricDefData,
};
use crate::objects::ObjectData;
use crate::entries::ObjectMetricData;

impl Hierarchy {
    pub fn add_object(
        &mut self,
        id: Option<String>,
        name: String,
        kind: NodeKind,
        parent: Option<ObjectIx>,
    ) -> Result<ObjectIx> {
        let ext_id = id.unwrap_or_else(|| format!("obj_{}", self.id_counter));
        self.id_counter += 1;
        if self.by_ext_object.contains_key(&ext_id) {
            return Err(crate::error::EngineError::ObjectNotFound(format!("duplicate id {ext_id}")));
        }
        let ix = ObjectIx(self.objects.len() as u32);
        self.objects.push(ObjectData {
            ix,
            id: ext_id.clone(),
            name,
            kind,
            parent,
            requests: Vec::new(),
            results: Vec::new(),
            alive: true,
        });
        self.children.push(Vec::new());
        self.by_ext_object.insert(ext_id, ix);
        self.live_objects += 1;
        Ok(ix)
    }

    pub fn remove_object_slot(&mut self, ix: ObjectIx) {
        let obj = &mut self.objects[ix.idx()];
        obj.alive = false;
        self.live_objects -= 1;
        if let Some(p) = obj.parent {
            if let Some(children) = self.children.get_mut(p.idx()) {
                children.retain(|&c| c != ix);
            }
        }
        self.by_ext_object.remove(&obj.id);
    }

    pub fn rename_object(&mut self, ix: ObjectIx, name: String) {
        self.objects[ix.idx()].name = name;
    }

    pub fn add_metric_def(
        &mut self,
        id: Option<String>,
        name: String,
        kind: MetricKind,
        unit: String,
        api: Option<crate::types::MetricApi>,
    ) -> Result<MetricDefIx> {
        let ext_id = id.unwrap_or_else(|| format!("md_{}", self.id_counter));
        self.id_counter += 1;
        if self.by_ext_metric_def.contains_key(&ext_id) {
            return Err(crate::error::EngineError::ObjectNotFound(format!("duplicate id {ext_id}")));
        }
        let ix = MetricDefIx(self.metric_defs.len() as u32);
        self.metric_defs.push(MetricDefData {
            ext_id: ext_id.clone(),
            name,
            kind,
            unit,
            api,
            alive: true,
        });
        self.by_ext_metric_def.insert(ext_id, ix);
        self.live_metric_defs += 1;
        Ok(ix)
    }

    pub fn remove_metric_def(&mut self, ix: MetricDefIx) {
        let md = &mut self.metric_defs[ix.idx()];
        md.alive = false;
        self.live_metric_defs -= 1;
        self.by_ext_metric_def.remove(&md.ext_id);
    }

    pub fn rename_metric_def(&mut self, ix: MetricDefIx, name: String) {
        self.metric_defs[ix.idx()].name = name;
    }

    pub fn set_metric_def_unit(&mut self, ix: MetricDefIx, unit: String) {
        self.metric_defs[ix.idx()].unit = unit;
    }

    pub fn add_object_metric(
        &mut self,
        oix: ObjectIx,
        dir: Direction,
        id: Option<String>,
        mdix: MetricDefIx,
        _ext_id: String,
    ) -> Result<EntryIx> {
        let entry_id = id.unwrap_or_else(|| format!("e_{}", self.id_counter));
        self.id_counter += 1;
        let eix = EntryIx(self.object_metrics.len() as u32);
        self.object_metrics.push(ObjectMetricData {
            ext_id: entry_id.clone(),
            object: oix,
            metric_def: mdix,
            direction: dir,
            alive: true,
            in_process: false,
        });
        self.by_ext_entry.insert((oix, entry_id), eix);
        let obj = &mut self.objects[oix.idx()];
        match dir {
            Direction::Request => obj.requests.push(eix),
            Direction::Result => obj.results.push(eix),
        }
        self.live_object_metrics += 1;
        self.ensure();
        Ok(eix)
    }

    pub fn remove_object_metric(&mut self, e: EntryIx) {
        let meta = &mut self.object_metrics[e.idx()];
        meta.alive = false;
        self.live_object_metrics -= 1;
        self.by_ext_entry.remove(&(meta.object, meta.ext_id.clone()));
        let obj = &mut self.objects[meta.object.idx()];
        match meta.direction {
            Direction::Request => obj.requests.retain(|&x| x != e),
            Direction::Result => obj.results.retain(|&x| x != e),
        }
    }

    pub fn reorder_object_metric(
        &mut self,
        oix: ObjectIx,
        dir: Direction,
        from_ix: usize,
        to_ix: usize,
    ) -> Result<()> {
        let list = match dir {
            Direction::Request => &mut self.objects[oix.idx()].requests,
            Direction::Result => &mut self.objects[oix.idx()].results,
        };
        if from_ix >= list.len() || to_ix >= list.len() {
            return Err(crate::error::EngineError::ObjectNotFound("index out of range".to_owned()));
        }
        let e = list.remove(from_ix);
        list.insert(to_ix, e);
        Ok(())
    }
}

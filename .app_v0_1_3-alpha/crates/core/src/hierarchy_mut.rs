//! Мутации иерархии: добавление/удаление объектов, показателей, записей, связей.

use crate::{
    error::Result,
    types::*,
    Hierarchy, MetricDefData,
};
use crate::edges::EdgeData;
use crate::objects::ObjectData;
use crate::entries::ObjectMetricData;

impl Hierarchy {
    pub fn add_object(
        &mut self,
        id: Option<String>,
        name: String,
        kind: NodeKind,
        parent: Option<ObjectIx>,
        direction: Option<ContainerDirection>,
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
            children: Vec::new(),
            direction,
            requests: Vec::new(),
            results: Vec::new(),
            entry_ix: None,
            alive: true,
        });
        self.children.push(Vec::new());
        self.by_ext_object.insert(ext_id, ix);
        self.live_objects += 1;
        // Добавляем объект в список детей родителя.
        if let Some(p) = parent {
            self.objects[p.idx()].children.push(ix);
        }
        Ok(ix)
    }

    pub fn remove_object_slot(&mut self, ix: ObjectIx) {
        let parent = self.objects[ix.idx()].parent;
        let id = self.objects[ix.idx()].id.clone();
        self.objects[ix.idx()].alive = false;
        self.live_objects -= 1;
        if let Some(p) = parent {
            if let Some(parent_obj) = self.objects.get_mut(p.idx()) {
                parent_obj.children.retain(|&c| c != ix);
            }
        }
        self.by_ext_object.remove(&id);
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
        name: String,
    ) -> Result<EntryIx> {
        let obj = &self.objects[oix.idx()];
        if obj.kind != NodeKind::Container {
            return Err(crate::error::EngineError::InvalidConnection(
                "метрику можно добавить только в Контейнер".to_owned(),
            ));
        }
        let count = obj
            .children
            .iter()
            .filter(|&c| self.objects[c.idx()].kind == NodeKind::Metric)
            .count();
        if count >= MAX_METRICS_PER_SIDE {
            return Err(crate::error::EngineError::ObjectNotFound(format!(
                "среди показов контейнера {} слишком много",
                obj.id
            )));
        }
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
        self.by_ext_entry.insert((oix, entry_id.clone()), eix);
        // Создаём объект-метрику в общем массиве объектов.
        let metric_ix = ObjectIx(self.objects.len() as u32);
        self.objects.push(ObjectData {
            ix: metric_ix,
            id: entry_id.clone(),
            name,
            kind: NodeKind::Metric,
            parent: Some(oix),
            children: Vec::new(),
            direction: None,
            requests: Vec::new(),
            results: Vec::new(),
            entry_ix: Some(eix),
            alive: true,
        });
        self.children.push(Vec::new());
        self.objects[oix.idx()].children.push(metric_ix);
        self.live_object_metrics += 1;
        self.live_objects += 1;
        self.ensure();
        Ok(eix)
    }

    pub fn remove_object_metric(&mut self, e: EntryIx) {
        let meta = &self.object_metrics[e.idx()];
        let container = meta.object;
        let ext_id = meta.ext_id.clone();
        // Находим объект-метрику по container+ext_id.
        let metric_ix = self
            .objects
            .iter()
            .position(|o| {
                o.alive
                    && o.kind == NodeKind::Metric
                    && o.parent == Some(container)
                    && o.id == ext_id
            })
            .map(|i| ObjectIx(i as u32));
        if let Some(mi) = metric_ix {
            self.objects[container.idx()]
                .children
                .retain(|&c| c != mi);
            // Помечаем объект-метрику как мёртвый.
            let m_obj = &mut self.objects[mi.idx()];
            m_obj.alive = false;
            m_obj.id = String::new();
            m_obj.name = String::new();
            m_obj.entry_ix = None;
            self.live_objects -= 1;
        }
        let om = &mut self.object_metrics[e.idx()];
        om.alive = false;
        self.live_object_metrics -= 1;
        self.by_ext_entry.remove(&(container, ext_id));
    }

    pub fn reorder_object_metric(
        &mut self,
        oix: ObjectIx,
        dir: Direction,
        from_ix: usize,
        to_ix: usize,
    ) -> Result<()> {
        let metric_children: Vec<ObjectIx> = self
            .objects
            .iter()
            .enumerate()
            .filter(|(_, o)| {
                o.alive
                    && o.kind == NodeKind::Metric
                    && o.parent == Some(oix)
                    && o.entry_ix.is_some()
                    && self.object_metrics[o.entry_ix.unwrap().idx()].direction == dir
            })
            .map(|(i, _)| ObjectIx(i as u32))
            .collect();
        if from_ix >= metric_children.len() || to_ix >= metric_children.len() || from_ix == to_ix {
            return Ok(());
        }
        let children_vec = &mut self.objects[oix.idx()].children;
        let from_pos = children_vec
            .iter()
            .position(|&c| c == metric_children[from_ix])
            .ok_or_else(|| crate::error::EngineError::ObjectNotFound("from_ix not found".to_owned()))?;
        let to_pos = children_vec
            .iter()
            .position(|&c| c == metric_children[to_ix])
            .ok_or_else(|| crate::error::EngineError::ObjectNotFound("to_ix not found".to_owned()))?;
        let entry = children_vec.remove(from_pos);
        children_vec.insert(to_pos, entry);
        Ok(())
    }

    pub fn connect(&mut self, from: EntryIx, to: EntryIx, throughput: f64) -> Result<EdgeIx> {
        let (fm, tm) = (
            &self.object_metrics[from.idx()],
            &self.object_metrics[to.idx()],
        );
        if fm.direction != Direction::Result || tm.direction != Direction::Request {
            return Err(crate::error::EngineError::InvalidConnection(
                "связь идёт только от записи Результата к записи Запроса".to_owned(),
            ));
        }
        if fm.object == tm.object {
            return Err(crate::error::EngineError::InvalidConnection(
                "объект нельзя подключить к самому себе".to_owned(),
            ));
        }
        if self.find_edge(from, to).is_some() {
            return Err(crate::error::EngineError::EdgeExists);
        }
        let data = EdgeData {
            ix: EdgeIx((self.edges.len()) as u32),
            from,
            to,
            throughput,
            alive: true,
        };
        let ix = EdgeIx((self.edges.len()) as u32);
        self.edges.push(data);
        self.live_edges += 1;
        Ok(ix)
    }

    pub fn remove_edge_internal(&mut self, edge: EdgeIx) {
        let e = &mut self.edges[edge.idx()];
        e.alive = false;
        self.live_edges -= 1;
    }
}

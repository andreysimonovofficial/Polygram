//! Формат файла `.pg` версии 2. Хранится только исходное: структура, значения, клетки.
//! Производное (статусы, потоки, маршруты линий) пересчитывается при открытии.
//! Файлы старых версий не открываются.

use crate::{
    command::Command,
    engine::Engine,
    error::{EngineError, Result},
    types::*,
};
use serde::{Deserialize, Serialize};

pub const FORMAT: &str = "polygram/2";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DocMetricDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub kind: MetricKind,
    #[serde(default)]
    pub unit: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api: Option<MetricApi>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DocObjectMetric {
    pub id: String,
    pub metric_def: String,
    #[serde(default)]
    pub dir: Direction,
    #[serde(default)]
    pub value: MetricValue,
    #[serde(default)]
    pub required: MetricValue,
    #[serde(default)]
    pub max: Option<MetricValue>,
    #[serde(default)]
    pub fact: MetricValue,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DocObject {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub kind: NodeKind,
    pub at: [i32; 2],
    #[serde(default)]
    pub parent: Option<String>,
    #[serde(default)]
    pub requests: Vec<DocObjectMetric>,
    #[serde(default)]
    pub results: Vec<DocObjectMetric>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DocEdge {
    pub from_object: String,
    pub from_entry: String,
    pub to_object: String,
    pub to_entry: String,
    #[serde(default = "default_edge_throughput")]
    pub throughput: f64,
}

fn default_edge_throughput() -> f64 {
    100.0
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub format: String,
    pub name: String,
    #[serde(default)]
    pub metric_defs: Vec<DocMetricDef>,
    #[serde(default)]
    pub objects: Vec<DocObject>,
    #[serde(default)]
    pub edges: Vec<DocEdge>,
}

impl Document {
    pub fn parse(json: &str) -> Result<Document> {
        let v: serde_json::Value =
            serde_json::from_str(json).map_err(|e| EngineError::Document(e.to_string()))?;
        match v.get("format").and_then(|f| f.as_str()) {
            Some(FORMAT) => {}
            Some(other) => return Err(EngineError::UnsupportedFormat(other.to_owned())),
            None => {
                return Err(EngineError::UnsupportedFormat(
                    "формат не указан".to_owned(),
                ))
            }
        }
        serde_json::from_value(v).map_err(|e| EngineError::Document(e.to_string()))
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("документ всегда сериализуется")
    }
}

impl Engine {
    pub fn to_document(&self) -> Document {
        let h = self.hierarchy();
        let v = self.values();
        let metric_defs = h
            .live_metric_defs()
            .map(|m| {
                let md = h.metric_def(m);
                DocMetricDef {
                    id: md.ext_id.clone(),
                    name: md.name.clone(),
                    kind: md.kind,
                    unit: md.unit.clone(),
                    api: md.api.clone(),
                }
            })
            .collect();
        let objects = h
            .live_objects()
            .map(|o| {
                let d = h.object(o);
                let r = self.layout().rect_of(o);
                let requests: Vec<DocObjectMetric> = d
                    .requests
                    .iter()
                    .map(|&e| {
                        let om = h.object_metric(e);
                        let md = h.metric_def(om.metric_def);
                        let i = e.idx();
                        DocObjectMetric {
                            id: om.ext_id.clone(),
                            metric_def: md.ext_id.clone(),
                            dir: om.direction,
                            value: v.value[i].clone(),
                            required: v.required[i].clone(),
                            max: v.max[i].clone(),
                            fact: v.fact[i].clone(),
                        }
                    })
                    .collect();
                let results: Vec<DocObjectMetric> = d
                    .results
                    .iter()
                    .map(|&e| {
                        let om = h.object_metric(e);
                        let md = h.metric_def(om.metric_def);
                        let i = e.idx();
                        DocObjectMetric {
                            id: om.ext_id.clone(),
                            metric_def: md.ext_id.clone(),
                            dir: om.direction,
                            value: v.value[i].clone(),
                            required: v.required[i].clone(),
                            max: v.max[i].clone(),
                            fact: v.fact[i].clone(),
                        }
                    })
                    .collect();
                DocObject {
                    id: d.ext_id.clone(),
                    name: d.name.clone(),
                    kind: d.kind,
                    at: [r.x, r.y],
                    parent: d.parent.map(|p| h.object(p).ext_id.clone()),
                    requests,
                    results,
                }
            })
            .collect();
        let edges = h
            .live_edges()
            .map(|e| {
                let d = h.edge(e);
                let (fm, tm) = (h.object_metric(d.from), h.object_metric(d.to));
                DocEdge {
                    from_object: h.object(fm.object).ext_id.clone(),
                    from_entry: fm.ext_id.clone(),
                    to_object: h.object(tm.object).ext_id.clone(),
                    to_entry: tm.ext_id.clone(),
                    throughput: d.throughput,
                }
            })
            .collect();
        Document {
            format: FORMAT.to_owned(),
            name: self.name().to_owned(),
            metric_defs,
            objects,
            edges,
        }
    }

    pub fn from_document(doc: &Document) -> Result<Engine> {
        if doc.format != FORMAT {
            return Err(EngineError::UnsupportedFormat(doc.format.clone()));
        }
        let mut engine = Engine::with_capacity(doc.objects.len(), doc.edges.len());
        engine.set_name(doc.name.clone());
        let mut cmds: Vec<Command> = Vec::new();
        // Сначала показатели-определения.
        for md in &doc.metric_defs {
            cmds.push(Command::AddMetricDef {
                id: Some(md.id.clone()),
                name: md.name.clone(),
                kind: md.kind,
                unit: md.unit.clone(),
                api: md.api.clone(),
            });
        }
        // Объекты в порядке зависимостей (родитель перед ребёнком), чтобы
        // find_object(parent) не падал при прямой ссылке на родителя,
        // который встречается позже в списке.
        let ordered_objects = Self::topological_order(doc);
        // Потом объекты.
        for o in &ordered_objects {
            cmds.push(Command::AddObject {
                id: Some(o.id.clone()),
                name: o.name.clone(),
                kind: o.kind,
                at: Some(o.at),
                parent: o.parent.clone(),
                author: None,
            });
            for (dir, list) in [
                (Direction::Request, &o.requests),
                (Direction::Result, &o.results),
            ] {
                for m in list {
                    cmds.push(Command::AddMetricEntry {
                        object: o.id.clone(),
                        dir,
                        id: Some(m.id.clone()),
                        metric_def: m.metric_def.clone(),
                        name: String::new(),
                        value: m.value.clone(),
                        required: m.required.clone(),
                        max: m.max.clone(),
                        fact: m.fact.clone(),
                    });
                }
            }
        }
        // Потом связи.
        for e in &doc.edges {
            cmds.push(Command::Connect {
                from_object: e.from_object.clone(),
                from_entry: e.from_entry.clone(),
                to_object: e.to_object.clone(),
                to_entry: e.to_entry.clone(),
                throughput: e.throughput,
            });
        }
        engine
            .apply_batch(Scope::Root, cmds)
            .map_err(|(i, e)| EngineError::Document(format!("команда {}: {e}", i + 1)))?;
        Ok(engine)
    }

/// Топологическая сортировка объектов: родители před детьми.
/// При циклической зависимости или отсутствующем родителе оставляет
/// оставшиеся объекты как есть (ошибкаявится на apply_batch).
fn topological_order(doc: &Document) -> Vec<&DocObject> {
    use std::collections::HashSet;
    let mut ordered: Vec<&DocObject> = Vec::new();
    let mut added: HashSet<&str> = HashSet::new();
    let mut remaining: Vec<&DocObject> = doc.objects.iter().collect();
    while !remaining.is_empty() {
        let mut progress = false;
        let mut next_remaining = Vec::new();
        for o in &remaining {
            if o.parent.as_ref().map_or(true, |p| added.contains(p.as_str())) {
                ordered.push(*o);
                added.insert(o.id.as_str());
                progress = true;
            } else {
                next_remaining.push(*o);
            }
        }
        if !progress {
            ordered.extend(remaining.into_iter());
            break;
        }
        remaining = next_remaining;
    }
    ordered
}

    pub fn from_json(json: &str) -> Result<Engine> {
        Engine::from_document(&Document::parse(json)?)
    }

    pub fn to_json(&self) -> String {
        self.to_document().to_json()
    }
}

//! Импорт документа (.pg JSON) как отдела (подграфа) в текущий граф.
//!
//! Единственная ответственность — переназначение ID, топологическая сортировка
//! и сборка пачки команд для вложения импортируемого отдела.

use polygram_core::{
    Command, Document, EngineError, types::{Direction, NodeKind},
};
use std::collections::HashMap;

use super::ApiService;
use polygram_core::document::DocObject;

impl ApiService {
    /// Импортировать документ (.pg JSON) как новый отдел (подграф) в текущий граф.
    /// Все импортируемые объекты помещаются в созданный отдел (с сохранением внутренней
    /// иерархии отделов и связей). ID объектов и показателей переназначаются с уникальным
    /// префиксом, чтобы избежать конфликтов с существующими в графе.
    pub fn import_document_as_department(
        &self,
        json: &str,
        dept_name: &str,
    ) -> Result<String, EngineError> {
        let doc = Document::parse(json)?;
        let handle = self.inner.runtime.handle();

        // Текущие ID объектов и показателей в графе (для проверки уникальности).
        let existing_object_ids: Vec<String> = handle
            .query(|e| {
                e.hierarchy()
                    .live_objects()
                    .map(|ix| e.hierarchy().object(ix).ext_id.clone())
                    .collect()
            })
            .unwrap_or_default();
        let existing_metric_def_ids: Vec<String> = handle
            .query(|e| {
                e.hierarchy()
                    .live_metric_defs()
                    .map(|ix| e.hierarchy().metric_def(ix).ext_id.clone())
                    .collect()
            })
            .unwrap_or_default();

        // Уникальный ID нового отдела.
        let mut used_object_ids = existing_object_ids.clone();
        let dept_id = unique_id("imp", &mut used_object_ids);

        // Переназначение ID показателей (глобальное пространство имён).
        let mut md_map: HashMap<String, String> = HashMap::new();
        let mut used_md_ids = existing_metric_def_ids.clone();
        for md in &doc.metric_defs {
            let new_id = unique_id(&md.id, &mut used_md_ids);
            md_map.insert(md.id.clone(), new_id.clone());
            used_md_ids.push(new_id);
        }

        // Топологическая сортировка объектов: родители перед детьми.
        let obj_order = topological_sort_objects(&doc.objects);

        // Переназначение ID объектов.
        let mut obj_map: HashMap<String, String> = HashMap::new();
        for oid in &obj_order {
            let o = doc
                .objects
                .iter()
                .find(|o| o.id == *oid)
                .expect("объект из топологической сортировки");
            let new_id = unique_id(&o.id, &mut used_object_ids);
            obj_map.insert(o.id.clone(), new_id.clone());
            used_object_ids.push(new_id);
        }

        // Построение пачки команд.
        let mut cmds: Vec<Command> = Vec::new();

        // 1. Сам отдел.
        cmds.push(Command::AddObject {
            id: Some(dept_id.clone()),
            name: dept_name.to_owned(),
            kind: NodeKind::Department,
            at: None,
            parent: None,
            author: None,
        });

        // 2. Показатели.
        for md in &doc.metric_defs {
            cmds.push(Command::AddMetricDef {
                id: Some(md_map[&md.id].clone()),
                name: md.name.clone(),
                kind: md.kind,
                unit: md.unit.clone(),
                api: md.api.clone(),
            });
        }

        // 3. Объекты (в топологическом порядке — родители перед детьми).
        for oid in &obj_order {
            let o = doc
                .objects
                .iter()
                .find(|o| o.id == *oid)
                .expect("объект из топологической сортировки");
            let new_parent = o
                .parent
                .as_ref()
                .and_then(|p| obj_map.get(p).cloned())
                .unwrap_or_else(|| dept_id.clone());
            cmds.push(Command::AddObject {
                id: Some(obj_map[oid].clone()),
                name: o.name.clone(),
                kind: o.kind,
                at: Some(o.at),
                parent: Some(new_parent),
                author: None,
            });
        }

        // 4. Записи показателей объектов (entry ID scoped к объекту — оставляем как есть,
        //    но metric_def переназначаем).
        for oid in &obj_order {
            let o = doc
                .objects
                .iter()
                .find(|o| o.id == *oid)
                .expect("объект из топологической сортировки");
            for (dir, list) in [
                (Direction::Request, &o.requests),
                (Direction::Output, &o.outputs),
            ] {
                for m in list {
                    cmds.push(Command::AddMetricEntry {
                        object: obj_map[oid].clone(),
                        dir,
                        id: Some(m.id.clone()),
                        metric_def: md_map[&m.metric_def].clone(),
                        name: m.id.clone(),
                        value: m.value.clone(),
                        required: m.required.clone(),
                        max: m.max.clone(),
                        fact: m.fact.clone(),
                    });
                }
            }
        }

        // 5. Связи (переназначены from_object / to_object, entry ID не меняются).
        for e in &doc.edges {
            if let (Some(from), Some(to)) = (
                obj_map.get(&e.from_object),
                obj_map.get(&e.to_object),
            ) {
                cmds.push(Command::Connect {
                    from_object: from.clone(),
                    from_entry: e.from_entry.clone(),
                    to_object: to.clone(),
                    to_entry: e.to_entry.clone(),
                    throughput: e.throughput,
                });
            }
        }

        handle
            .apply_batch(cmds)
            .map(|_| dept_id.clone())
            .map_err(|(_, e)| e)?;
        Ok(dept_id)
    }
}

/// Сгенерировать уникальный ID, которого нет в списке `used`.
/// Если `base` свободен — возвращает его как есть, иначе добавляет суффикс `_1`, `_2`, ...
fn unique_id(base: &str, used: &mut Vec<String>) -> String {
    if !used.contains(&base.to_owned()) {
        used.push(base.to_owned());
        return base.to_owned();
    }
    let mut n = 1u64;
    loop {
        let candidate = format!("{base}_{n}");
        if !used.contains(&candidate) {
            used.push(candidate.clone());
            return candidate;
        }
        n += 1;
    }
}

/// Топологическая сортировка объектов документа: родители перед детьми.
/// Объекты, чей родитель не входит в документ (или None), идут первыми.
fn topological_sort_objects(objects: &[DocObject]) -> Vec<String> {
    let id_set: std::collections::HashSet<&str> = objects
        .iter()
        .map(|o| o.id.as_str())
        .collect();
    let mut result: Vec<String> = Vec::new();
    let mut visited: std::collections::HashSet<&str> = std::collections::HashSet::new();

    fn visit<'a>(
        oid: &'a str,
        objects: &'a [DocObject],
        id_set: &std::collections::HashSet<&str>,
        visited: &mut std::collections::HashSet<&'a str>,
        result: &mut Vec<String>,
    ) {
        if !visited.insert(oid) {
            return;
        }
        let obj = match objects.iter().find(|o| o.id == oid) {
            Some(o) => o,
            None => return,
        };
        // Сначала посещаем родителя (если он в документе).
        if let Some(parent) = &obj.parent {
            if id_set.contains(parent.as_str()) {
                visit(parent, objects, id_set, visited, result);
            }
        }
        result.push(oid.to_owned());
    }

    for o in objects {
        visit(&o.id, objects, &id_set, &mut visited, &mut result);
    }
    result
}

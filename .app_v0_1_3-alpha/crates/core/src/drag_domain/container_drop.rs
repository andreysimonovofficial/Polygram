//! Расчёт команд при дропе контейнера (Запрос/Результат).

use crate::command::Command;
use crate::geometry::{Geometry, ObjectGeom};
use crate::types::{ContainerDirection, NodeKind};
use crate::drag_domain::free_cell::{find_free_cell_in_dept_domain, opposite_dir};
use crate::MetricValue;

/// Результат расчёта команд при дропе контейнера.
pub struct ContainerDropResult {
    pub commands: Vec<Command>,
    pub final_cell: (i32, i32),
    pub reparent_dept: Option<String>,
}

/// Рассчитать команды при дропе контейнера (Запрос/Результат).
/// Создаёт Процесс (если цель не процесс), копирует показатели контейнера в контейнер целевого объекта с противоположным направлением, связывает показатели.
pub fn compute_container_drop(
    geometry: &Geometry,
    dragged_id: &str,
    dragged_dir: ContainerDirection,
    start_cell: (i32, i32),
    drop_cell: (i32, i32),
    over_dept: Option<&str>,
    dragged_size: (i32, i32),
    target: Option<&ObjectGeom>,
) -> ContainerDropResult {
    let target_dir = opposite_dir(dragged_dir);
    let existing_ids: Vec<String> = geometry.objects.iter().map(|o| o.id.clone()).collect();
    let process_id = crate::id_allocator::allocate_id(&existing_ids);

    // Данные dragged-контейнера.
    let dragged_obj = geometry.objects.iter().find(|o| o.id == dragged_id).unwrap();
    let source_entries = match dragged_dir {
        ContainerDirection::Request => &dragged_obj.requests,
        ContainerDirection::Result => &dragged_obj.results,
    };

    // Контейнер без показателей — отмена.
    if source_entries.is_empty() {
        return ContainerDropResult {
            commands: Vec::new(),
            final_cell: start_cell,
            reparent_dept: None,
        };
    }

    // Определяем целевую клетку для нового процесса.
    let final_cell = if let Some(dept_id) = over_dept {
        if geometry.objects.iter().any(|o| o.id == dept_id && o.kind == NodeKind::Department) {
            find_free_cell_in_dept_domain(geometry, dept_id, dragged_size.0, dragged_size.1, dragged_id)
                .unwrap_or(drop_cell)
        } else {
            drop_cell
        }
    } else {
        drop_cell
    };

    let mut commands: Vec<Command> = Vec::new();
    let target_object_id: String;

    // Определяем целевой объект для копирования показателей.
    match target {
        Some(t) if t.kind == NodeKind::Process => {
            // Цель — процесс: не создаём новый процесс, копируем показатели в его контейнер.
            target_object_id = t.id.clone();
        }
        _ if over_dept.is_some() => {
            // Над отделом: создаём новый процесс.
            commands.push(Command::AddObject {
                id: Some(process_id.clone()),
                name: format!("Процесс {}", process_id),
                kind: NodeKind::Process,
                at: Some([final_cell.0, final_cell.1]),
                parent: None,
                direction: None,
                author: None,
            });
            target_object_id = process_id.clone();

            // Привязываем родителя Отдел к новому Процессу.
            if let Some(dept_id) = over_dept {
                if geometry.objects.iter().any(|o| o.id == dept_id && o.kind == NodeKind::Department) {
                    commands.push(Command::SetObjectParent {
                        object: dept_id.to_owned(),
                        parent: Some(process_id.clone()),
                    });
                }
            }
        }
        _ if target.is_none() => {
            // Свободное место: создаём новый процесс.
            commands.push(Command::AddObject {
                id: Some(process_id.clone()),
                name: format!("Процесс {}", process_id),
                kind: NodeKind::Process,
                at: Some([final_cell.0, final_cell.1]),
                parent: None,
                direction: None,
                author: None,
            });
            target_object_id = process_id.clone();
        }
        _ => {
            // Иная цель (контейнер, показатель, иное) — отмена.
            return ContainerDropResult {
                commands: Vec::new(),
                final_cell: start_cell,
                reparent_dept: None,
            };
        }
    }

    // Копируем показатели контейнера в контейнер целевого объекта с противоположным направлением.
    for entry in source_entries {
        commands.push(Command::AddMetricEntry {
            object: target_object_id.clone(),
            dir: target_dir.into_direction(),
            id: Some(entry.name.clone()),
            metric_def: entry.name.clone(),
            name: entry.name.clone(),
            value: MetricValue::Null,
            required: MetricValue::Null,
            max: None,
            fact: MetricValue::Null,
        });
        commands.push(Command::Connect {
            from_object: dragged_id.to_owned(),
            from_entry: entry.name.clone(),
            to_object: target_object_id.clone(),
            to_entry: entry.name.clone(),
            throughput: 100.0,
        });
    }

    ContainerDropResult {
        commands,
        final_cell,
        reparent_dept: over_dept.map(|s| s.to_owned()),
    }
}

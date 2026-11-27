//! Расчёт команд при дропе показателя (Запрос/Результат) как объекта.

use crate::command::Command;
use crate::geometry::{Geometry, ObjectGeom};
use crate::types::{ContainerDirection, NodeKind};
use crate::drag_domain::free_cell::{find_free_cell_in_dept_domain, opposite_dir};
use crate::MetricValue;

/// Результат расчёта команд при дропе показателя как объекта.
pub struct MetricDropObjectResult {
    pub commands: Vec<Command>,
    pub final_cell: (i32, i32),
    pub reparent_dept: Option<String>,
}

/// Рассчитать команды при дропе показателя (Запрос/Результат).
/// Создаёт Процесс (если нужно), создаёт показатель в контейнере процесса с противоположным направлением, связывает показатели.
pub fn compute_metric_drop_object(
    geometry: &Geometry,
    // Объект, чья запись-источник используется в связи (для перетаскиваемого объекта-Показателя — его id;
    // для строки Показателя внутри Процесса — id этого Процесса).
    source_object: &str,
    // Имя записи-показателя (используется как имя новой записи в цели и ключ связи).
    entry_name: &str,
    dragged_dir: ContainerDirection,
    start_cell: (i32, i32),
    drop_cell: (i32, i32),
    over_dept: Option<&str>,
    dragged_size: (i32, i32),
    target: Option<&ObjectGeom>,
) -> MetricDropObjectResult {
    let target_dir = opposite_dir(dragged_dir);
    let existing_ids: Vec<String> = geometry.objects.iter().map(|o| o.id.clone()).collect();
    let process_id = crate::id_allocator::allocate_id(&existing_ids);

    // Определяем целевую клетку для нового процесса (или показателя).
    let final_cell = if let Some(dept_id) = over_dept {
        if geometry.objects.iter().any(|o| o.id == dept_id && o.kind == NodeKind::Department) {
            find_free_cell_in_dept_domain(geometry, dept_id, dragged_size.0, dragged_size.1, source_object)
                .unwrap_or(drop_cell)
        } else {
            drop_cell
        }
    } else {
        drop_cell
    };

    let mut commands: Vec<Command> = Vec::new();
    let target_object_id: String;
    let mut push_entry = true;

    // Определяем целевой объект для добавления показателя.
    match target {
        Some(t) if t.kind == NodeKind::Process => {
            // Цель — процесс: добавляем показатель в его контейнер.
            target_object_id = t.id.clone();
        }
        Some(t) if t.kind == NodeKind::Container => {
            // Цель — контейнер: добавляем показатель в него (только если направление совпадает с целевым).
            // Если направление контейнера не совпадает с target_dir — отмена.
            if t.direction != Some(target_dir) {
                return MetricDropObjectResult {
                    commands: Vec::new(),
                    final_cell: start_cell,
                    reparent_dept: None,
                };
            }
            target_object_id = t.id.clone();
        }
        Some(t) if t.kind == NodeKind::Metric => {
            // Цель — показатель: только связываем показатели, новую запись не создаём.
            target_object_id = t.id.clone();
            push_entry = false;
        }
        _ => {
            // Отдел (через over_dept) или Свободное место: создаём новый процесс.
            // Иная цель (иное) — отмена.
            if let Some(dept_id) = over_dept {
                if geometry.objects.iter().any(|o| o.id == dept_id && o.kind == NodeKind::Department) {
                    commands.push(Command::AddObject {
                        id: Some(process_id.clone()),
                        name: format!("Процесс {}", process_id),
                        kind: NodeKind::Process,
                        at: Some([final_cell.0, final_cell.1]),
                        parent: None,
                        direction: None,
                        author: None,
                    });
                    commands.push(Command::SetObjectParent {
                        object: dept_id.to_owned(),
                        parent: Some(process_id.clone()),
                    });
                    target_object_id = process_id.clone();
                } else {
                    return MetricDropObjectResult {
                        commands: Vec::new(),
                        final_cell: start_cell,
                        reparent_dept: None,
                    };
                }
            } else if target.is_none() {
                // Свободное место.
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
            } else {
                // Иная цель (показатель, иное) — отмена.
                return MetricDropObjectResult {
                    commands: Vec::new(),
                    final_cell: start_cell,
                    reparent_dept: None,
                };
            }
        }
    }

    if push_entry {
        // Создать показатель в целевом контейнере с противоположным направлением.
        commands.push(Command::AddMetricEntry {
            object: target_object_id.clone(),
            dir: target_dir.into_direction(),
            id: Some(entry_name.to_owned()),
            metric_def: entry_name.to_owned(),
            name: entry_name.to_owned(),
            value: MetricValue::Null,
            required: MetricValue::Null,
            max: None,
            fact: MetricValue::Null,
        });
    }

    // Связать показатели: сторона Результата -> сторону Запроса.
    // При перетаскивании Запроса источникとなる цель — Результат, цель — Запрос (источник).
    let (from_object, from_entry, to_object, to_entry) = match dragged_dir {
        ContainerDirection::Result => (
            source_object.to_owned(),
            entry_name.to_owned(),
            target_object_id.clone(),
            entry_name.to_owned(),
        ),
        ContainerDirection::Request => (
            target_object_id.clone(),
            entry_name.to_owned(),
            source_object.to_owned(),
            entry_name.to_owned(),
        ),
    };
    commands.push(Command::Connect {
        from_object,
        from_entry,
        to_object,
        to_entry,
        throughput: 100.0,
    });

    MetricDropObjectResult {
        commands,
        final_cell,
        reparent_dept: over_dept.map(|s| s.to_owned()),
    }
}

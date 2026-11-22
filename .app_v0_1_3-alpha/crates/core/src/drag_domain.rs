//! Чистая доменная логика перетаскивания объектов и показателей на холсте.
//! Без egui, без окна, без сети. Только входное состояние и чистый результат.

use crate::geometry::Geometry;
use crate::layout::Rect;
use crate::types::{ContainerDirection, Direction, NodeKind};
use crate::{Command, MetricValue};

/// Вход для расчёта команд при дропе показателя (метрики).
pub struct MetricDropContext {
    /// Геометрия с наложенными предсказаниями.
    pub geometry: Geometry,
    /// Объект-источник показателя.
    pub source_object: String,
    /// Направление показателя у источника.
    pub source_dir: Direction,
    /// Имя показываемой записи.
    pub entry_name: String,
    /// Целевой объект.
    pub target_id: String,
}

/// Рассчитать команды при дропе показателя: добавить запись в цель (если нет) и создать связь.
pub fn compute_metric_drop(ctx: &MetricDropContext) -> Vec<Command> {
    let MetricDropContext {
        geometry,
        source_object,
        source_dir,
        entry_name,
        target_id,
    } = ctx;

    if target_id == source_object {
        return Vec::new();
    }

    let target = match geometry.objects.iter().find(|o| o.id == *target_id) {
        Some(t) if t.kind == NodeKind::Process => t,
        _ => return Vec::new(),
    };

    let target_dir = match source_dir {
        Direction::Request => Direction::Result,
        Direction::Result => Direction::Request,
    };

    let entries = match target_dir {
        Direction::Request => &target.requests,
        Direction::Result => &target.results,
    };

    let already_has = entries.iter().any(|e| e.name == *entry_name);
    let mut cmds: Vec<Command> = Vec::new();

    if !already_has {
        cmds.push(Command::AddMetricEntry {
            object: target_id.clone(),
            dir: target_dir,
            id: Some(entry_name.clone()),
            metric_def: entry_name.clone(),
            name: entry_name.clone(),
            value: MetricValue::Null,
            required: MetricValue::Null,
            max: None,
            fact: MetricValue::Null,
        });
    }

    let (from_object, from_entry, to_object, to_entry) = match source_dir {
        Direction::Result => (
            source_object.clone(),
            entry_name.clone(),
            target_id.clone(),
            entry_name.clone(),
        ),
        Direction::Request => (
            target_id.clone(),
            entry_name.clone(),
            source_object.clone(),
            entry_name.clone(),
        ),
    };

    cmds.push(Command::Connect {
        from_object,
        from_entry,
        to_object,
        to_entry,
        throughput: 100.0,
    });

    cmds
}

/// Направление, противоположное заданному.
fn opposite_dir(dir: ContainerDirection) -> ContainerDirection {
    match dir {
        ContainerDirection::Request => ContainerDirection::Result,
        ContainerDirection::Result => ContainerDirection::Request,
    }
}

/// Найти свободную клетку внутри отдела для объекта размером w×h клеток.
fn find_free_cell_in_dept_domain(
    geometry: &Geometry,
    dept_id: &str,
    w: i32,
    h: i32,
    exclude_id: &str,
) -> Option<(i32, i32)> {
    let dept = geometry.objects.iter().find(|o| o.id == dept_id && o.kind == NodeKind::Department)?;
    let inner_x0 = dept.rect.x + 1;
    let inner_y0 = dept.rect.y + 2;
    let inner_x1 = dept.rect.x + dept.rect.w - 1;
    let inner_y1 = dept.rect.y + dept.rect.h - 1;
    if inner_x1 < inner_x0 + w || inner_y1 < inner_y0 + h {
        return None;
    }
    for y in inner_y0..=inner_y1.saturating_sub(h) {
        for x in inner_x0..=inner_x1.saturating_sub(w) {
            let r = Rect::new(x, y, w, h);
            let free = geometry.objects.iter().all(|o| {
                o.id == exclude_id || o.id == dept_id || !o.rect.overlaps(&r)
            });
            if free {
                return Some((x, y));
            }
        }
    }
    None
}

/// Поиск объекта-цели под клеткой (исключая dragged_id).
fn target_object_at<'a>(geometry: &'a Geometry, cx: i32, cy: i32, exclude_id: &str) -> Option<&'a crate::geometry::ObjectGeom> {
    geometry.objects.iter().find(|o| {
        o.id != exclude_id
            && o.rect.x <= cx && cx < o.rect.x + o.rect.w
            && o.rect.y <= cy && cy < o.rect.y + o.rect.h
    })
}

/// Входное состояние для расчёта команд при дропе любого объекта (Отдел, Процесс, Контейнер, Показатель).
pub struct ObjectDropContext {
    pub geometry: Geometry,
    pub dragged_id: String,
    pub start_cell: (i32, i32),
    pub drop_cell: (i32, i32),
    pub over_dept: Option<String>,
    pub dragged_size: (i32, i32),
    /// Направление контейнера при перетаскивании шапки контейнера из карточки процесса.
    /// Если Some — используется логика контейнера независимо от NodeKind dragged_id.
    pub dragged_dir: Option<ContainerDirection>,
}

/// Результат расчёта команд при дропе объекта.
pub struct ObjectDropResult {
    pub commands: Vec<Command>,
    pub final_cell: (i32, i32),
    pub reparent_dept: Option<String>,
}

/// Рассчитать команды при дропе любого объекта на холсте.
pub fn compute_object_drop_commands(ctx: &ObjectDropContext) -> ObjectDropResult {
    let ObjectDropContext {
        geometry,
        dragged_id,
        start_cell,
        drop_cell,
        over_dept,
        dragged_size,
        dragged_dir,
    } = ctx;
    let start_cell = *start_cell;
    let drop_cell = *drop_cell;
    let dragged_size = *dragged_size;

    let dragged = match geometry.objects.iter().find(|o| o.id == dragged_id.as_str()) {
        Some(d) => d,
        None => return ObjectDropResult {
            commands: Vec::new(),
            final_cell: start_cell,
            reparent_dept: None,
        },
    };

    let over_dept_str = over_dept.as_deref();
    let target = target_object_at(geometry, drop_cell.0, drop_cell.1, dragged_id);

    // Если перетаскивается шапка контейнера (из карточки процесса или автономная),
    // используется логика контейнера независимо от NodeKind dragged_id.
    if let Some(dir) = dragged_dir {
        return compute_container_drop(geometry, dragged_id, *dir, start_cell, drop_cell, over_dept_str, dragged_size, target);
    }

    match dragged.kind {
        NodeKind::Department => compute_department_drop(geometry, dragged_id, start_cell, drop_cell, over_dept_str, dragged_size, target),
        NodeKind::Process => compute_process_drop(geometry, dragged_id, start_cell, drop_cell, over_dept_str, dragged_size, target),
        NodeKind::Container => {
            let dir = dragged.direction.unwrap_or(ContainerDirection::Request);
            compute_container_drop(geometry, dragged_id, dir, start_cell, drop_cell, over_dept_str, dragged_size, target)
        }
        NodeKind::Metric => {
            let dir = dragged.direction.unwrap_or(ContainerDirection::Request);
            compute_metric_drop_object(geometry, dragged_id, dir, start_cell, drop_cell, over_dept_str, dragged_size, target)
        }
    }
}

fn compute_department_drop(
    geometry: &Geometry,
    dragged_id: &str,
    start_cell: (i32, i32),
    drop_cell: (i32, i32),
    over_dept: Option<&str>,
    dragged_size: (i32, i32),
    target: Option<&crate::geometry::ObjectGeom>,
) -> ObjectDropResult {
    // Над отделом — репарентинг (модал обрабатывается на UI-слое).
    if let Some(dept_id) = over_dept {
        if geometry.objects.iter().any(|o| o.id == dept_id && o.kind == NodeKind::Department) {
            let final_cell = if target.map_or(false, |t| t.kind == NodeKind::Department) {
                find_free_cell_in_dept_domain(geometry, dept_id, dragged_size.0, dragged_size.1, dragged_id)
                    .unwrap_or(drop_cell)
            } else {
                drop_cell
            };
            return ObjectDropResult {
                commands: vec![
                    Command::SetObjectParent { object: dragged_id.to_owned(), parent: Some(dept_id.to_owned()) },
                    Command::MoveObject { object: dragged_id.to_owned(), at: [final_cell.0, final_cell.1] },
                ],
                final_cell,
                reparent_dept: Some(dept_id.to_owned()),
            };
        }
    }
    // Свободная клетка — просто перемещение.
    if drop_cell != start_cell {
        return ObjectDropResult {
            commands: vec![Command::MoveObject { object: dragged_id.to_owned(), at: [drop_cell.0, drop_cell.1] }],
            final_cell: drop_cell,
            reparent_dept: None,
        };
    }
    ObjectDropResult {
        commands: Vec::new(),
        final_cell: start_cell,
        reparent_dept: None,
    }
}

/// Рассчитать команды при дропе процесса.
fn compute_process_drop(
    geometry: &Geometry,
    dragged_id: &str,
    start_cell: (i32, i32),
    drop_cell: (i32, i32),
    over_dept: Option<&str>,
    dragged_size: (i32, i32),
    target: Option<&crate::geometry::ObjectGeom>,
) -> ObjectDropResult {
    // Над отделом — репарентинг (модал обрабатывается на UI-слое).
    if let Some(dept_id) = over_dept {
        if geometry.objects.iter().any(|o| o.id == dept_id && o.kind == NodeKind::Department) {
            let final_cell = if target.map_or(false, |t| t.kind == NodeKind::Department) {
                find_free_cell_in_dept_domain(geometry, dept_id, dragged_size.0, dragged_size.1, dragged_id)
                    .unwrap_or(drop_cell)
            } else {
                drop_cell
            };
            return ObjectDropResult {
                commands: vec![
                    Command::SetObjectParent { object: dragged_id.to_owned(), parent: Some(dept_id.to_owned()) },
                    Command::MoveObject { object: dragged_id.to_owned(), at: [final_cell.0, final_cell.1] },
                ],
                final_cell,
                reparent_dept: Some(dept_id.to_owned()),
            };
        }
    }
    // Свободная клетка — просто перемещение.
    if drop_cell != start_cell {
        return ObjectDropResult {
            commands: vec![Command::MoveObject { object: dragged_id.to_owned(), at: [drop_cell.0, drop_cell.1] }],
            final_cell: drop_cell,
            reparent_dept: None,
        };
    }
    ObjectDropResult {
        commands: Vec::new(),
        final_cell: start_cell,
        reparent_dept: None,
    }
}

/// Рассчитать команды при дропе контейнера (Запрос/Результат).
/// Создаёт Процесс (если цель не процесс), копирует показатели контейнера в контейнер целевого объекта с противоположным направлением, связывает показатели.
fn compute_container_drop(
    geometry: &Geometry,
    dragged_id: &str,
    dragged_dir: ContainerDirection,
    start_cell: (i32, i32),
    drop_cell: (i32, i32),
    over_dept: Option<&str>,
    dragged_size: (i32, i32),
    target: Option<&crate::geometry::ObjectGeom>,
) -> ObjectDropResult {
    let target_dir = opposite_dir(dragged_dir);
    let process_id = format!("proc_from_{}", dragged_id);

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
                name: process_id.clone(),
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
                name: process_id.clone(),
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
            return ObjectDropResult {
                commands: Vec::new(),
                final_cell: start_cell,
                reparent_dept: None,
            };
        }
    }

    // Копируем показатели контейнера в контейнер целевого объекта с противоположным направлением.
    let dragged_obj = geometry.objects.iter().find(|o| o.id == dragged_id).unwrap();
    let source_entries = match dragged_dir {
        ContainerDirection::Request => &dragged_obj.requests,
        ContainerDirection::Result => &dragged_obj.results,
    };

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

    ObjectDropResult {
        commands,
        final_cell,
        reparent_dept: over_dept.map(|s| s.to_owned()),
    }
}

/// Рассчитать команды при дропе показателя (Запрос/Результат).
/// Создаёт Процесс (если нужно), создаёт показатель в контейнере процесса с противоположным направлением, связывает показатели.
fn compute_metric_drop_object(
    geometry: &Geometry,
    dragged_id: &str,
    dragged_dir: ContainerDirection,
    start_cell: (i32, i32),
    drop_cell: (i32, i32),
    over_dept: Option<&str>,
    dragged_size: (i32, i32),
    target: Option<&crate::geometry::ObjectGeom>,
) -> ObjectDropResult {
    let target_dir = opposite_dir(dragged_dir);
    let process_id = format!("proc_from_{}", dragged_id);

    // Определяем целевую клетку.
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
                return ObjectDropResult {
                    commands: Vec::new(),
                    final_cell: start_cell,
                    reparent_dept: None,
                };
            }
            target_object_id = t.id.clone();
        }
        _ => {
            // Отдел (через over_dept) или Свободное место: создаём новый процесс.
            // Иная цель (показатель, иное) — отмена.
            if let Some(dept_id) = over_dept {
                if geometry.objects.iter().any(|o| o.id == dept_id && o.kind == NodeKind::Department) {
                    commands.push(Command::AddObject {
                        id: Some(process_id.clone()),
                        name: process_id.clone(),
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
                    return ObjectDropResult {
                        commands: Vec::new(),
                        final_cell: start_cell,
                        reparent_dept: None,
                    };
                }
            } else if target.is_none() {
                // Свободное место.
                commands.push(Command::AddObject {
                    id: Some(process_id.clone()),
                    name: process_id.clone(),
                    kind: NodeKind::Process,
                    at: Some([final_cell.0, final_cell.1]),
                    parent: None,
                    direction: None,
                    author: None,
                });
                target_object_id = process_id.clone();
            } else {
                // Иная цель (показатель, иное) — отмена.
                return ObjectDropResult {
                    commands: Vec::new(),
                    final_cell: start_cell,
                    reparent_dept: None,
                };
            }
        }
    }

    // Создать показатель в целевом контейнере с противоположным направлением.
    commands.push(Command::AddMetricEntry {
        object: target_object_id.clone(),
        dir: target_dir.into_direction(),
        id: Some(dragged_id.to_owned()),
        metric_def: dragged_id.to_owned(),
        name: dragged_id.to_owned(),
        value: MetricValue::Null,
        required: MetricValue::Null,
        max: None,
        fact: MetricValue::Null,
    });

    // Связать показатели: источник -> цель.
    commands.push(Command::Connect {
        from_object: dragged_id.to_owned(),
        from_entry: dragged_id.to_owned(),
        to_object: target_object_id,
        to_entry: dragged_id.to_owned(),
        throughput: 100.0,
    });

    ObjectDropResult {
        commands,
        final_cell,
        reparent_dept: over_dept.map(|s| s.to_owned()),
    }
}

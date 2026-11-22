//! Чистая доменная логика перетаскивания объектов и показателей на холсте.
//! Без egui, без окна, без сети. Только входное состояние и чистый результат.

use crate::geometry::Geometry;
use crate::layout::Rect;
use crate::types::{ContainerDirection, Direction, NodeKind};
use crate::{Command, MetricValue};

/// Входное состояние для решения о дропе перетаскиваемого объекта.
pub struct DropContext {
    /// Геометрия с наложенными клиентскими предсказаниями позиций.
    pub geometry: Geometry,
    /// ID перетаскиваемого объекта.
    pub dragged_id: String,
    /// Клетка, с которой началось перетаскивание.
    pub start_cell: (i32, i32),
    /// Клетка, куда объект должен попасть при дропе (призрачная или предсказанная).
    pub drop_cell: (i32, i32),
    /// Отдел, над которым нависает объект при дропе (None — не над отделом).
    pub over_dept: Option<String>,
    /// Найдена ли свободная призрачная клетка для дропа.
    pub has_ghost: bool,
    /// Размер перетаскиваемого объекта (ширина, высота в клетках).
    pub dragged_size: (i32, i32),
}

/// Результат решения о дропе.
#[derive(Clone, Debug, PartialEq)]
pub enum DropOutcome {
    /// Объект остаётся на своей стартовой клетке (отказ).
    Cancel,
    /// Просто переместить объект на указанную клетку.
    Move { at: (i32, i32) },
    /// Переместить объект в отдел и на указанную клетку.
    ReparentAndMove { dept_id: String, at: (i32, i32) },
}

/// Проверить, является ли объект с заданным id отделом.
fn is_department(geometry: &Geometry, id: &str) -> bool {
    geometry.objects.iter().any(|o| o.id == id && o.kind == NodeKind::Department)
}

/// Решить, что делать с перетаскиваемым объектом при отпускании кнопки мыши.
///
/// Правила:
/// 1. Если над отделом, но нет свободной призрачной клетки — Cancel (место занято).
/// 2. Если целевая клетка занята другим объектом (кроме целевого отдела при репарентинге) — Cancel.
/// 3. Если цель — отдел и есть призрачная клетка — ReparentAndMove.
/// 4. Если клетка не изменилась — Cancel.
/// 5. Иначе — Move.
pub fn compute_drop_outcome(ctx: &DropContext) -> DropOutcome {
    let DropContext {
        geometry,
        dragged_id,
        start_cell,
        drop_cell,
        over_dept,
        has_ghost,
        dragged_size,
    } = ctx;

    // 1. Над отделом, но нет свободной призрачной клетки — отказ.
    if over_dept.as_ref().map_or(false, |d| is_department(geometry, d)) && !*has_ghost {
        return DropOutcome::Cancel;
    }

    let (w, h) = *dragged_size;
    let drop_rect = Rect::new(drop_cell.0, drop_cell.1, w, h);

    // 2. Цель занята другим объектом. Целевой отдел исключается (он растянется под ребёнка).
    let overlaps = geometry.objects.iter().any(|o| {
        o.id != *dragged_id
            && o.rect.overlaps(&drop_rect)
            && !over_dept.as_ref().map_or(false, |d| o.id == *d)
    });
    if overlaps {
        return DropOutcome::Cancel;
    }

    // 3. Цель — отдел.
    if let Some(dept_id) = over_dept {
        if is_department(geometry, dept_id) {
            return DropOutcome::ReparentAndMove {
                dept_id: dept_id.clone(),
                at: *drop_cell,
            };
        }
    }

    // 4. Позиция не изменилась.
    if *drop_cell == *start_cell {
        return DropOutcome::Cancel;
    }

    // 5. Простой переезд.
    DropOutcome::Move { at: *drop_cell }
}

/// Найти объект, чей rect содержит клетку (cx, cy), исключая exclude_id.
/// Сначала проверяются не-департаменты, затем департаменты (как в оригинале).
pub fn object_at_cell<'a>(geometry: &'a Geometry, cx: i32, cy: i32, exclude_id: &str) -> Option<&'a str> {
    // Сначала не-департаменты.
    for object in &geometry.objects {
        if object.kind == NodeKind::Department {
            continue;
        }
        if object.id == exclude_id {
            continue;
        }
        if object.rect.x <= cx && cx < object.rect.x + object.rect.w
            && object.rect.y <= cy && cy < object.rect.y + object.rect.h
        {
            return Some(&object.id);
        }
    }
    // Затем самый вложенный отдел (чтобы подотдел возвращался, а не родительский).
    innermost_dept_at(geometry, cx, cy, exclude_id)
}

/// Найти самый вложенный отдел, содержащий клетку (cx, cy).
/// Если нескольких отделов содержат клетку, возвращается тот с наименьшим rect (самый глубокий в иерархии).
fn innermost_dept_at<'a>(geometry: &'a Geometry, cx: i32, cy: i32, exclude_id: &str) -> Option<&'a str> {
    let mut containing: Vec<&crate::geometry::ObjectGeom> = geometry
        .objects
        .iter()
        .filter(|o| o.kind == NodeKind::Department && o.id != exclude_id)
        .filter(|o| {
            o.rect.x <= cx && cx < o.rect.x + o.rect.w
                && o.rect.y <= cy && cy < o.rect.y + o.rect.h
        })
        .collect();
    if containing.is_empty() {
        return None;
    }
    containing.sort_by_key(|o| o.rect.w * o.rect.h);
    containing.first().map(|o| o.id.as_str())
}

/// Проверить, свободен ли rect (без учёта exclude_id) — нет пересечений с другими объектами.
pub fn rect_is_free(geometry: &Geometry, rect: &Rect, exclude_id: &str) -> bool {
    for object in &geometry.objects {
        if object.id == exclude_id {
            continue;
        }
        if object.rect.overlaps(rect) {
            return false;
        }
    }
    true
}

/// Вернуть true, если клетка (cx, cy) внутри отдела dept_id.
pub fn inside_dept(geometry: &Geometry, dept_id: &str, cx: i32, cy: i32) -> bool {
    geometry.objects.iter().find(|o| o.id == dept_id).map_or(false, |d| {
        d.rect.x <= cx && cx < d.rect.x + d.rect.w && d.rect.y <= cy && cy < d.rect.y + d.rect.h
    })
}

/// Свободная клетка для призрачной копии на расстоянии `offset` клеток от края объекта obj.
/// Пытаемся 4 стороны (справа, слева, снизу, сверху). Если ни одна не свободна —
/// спиральный поиск от центра объекта, начиная снаружи от его границ.
/// Если dept_id задан — ищем только внутри этого отдела.
pub fn find_ghost_cell(
    geometry: &Geometry,
    obj: &crate::geometry::ObjectGeom,
    ghost_w: i32,
    ghost_h: i32,
    offset: i32,
    exclude_id: &str,
    dept_id: Option<&str>,
) -> Option<(i32, i32)> {
    let r_obj = &obj.rect;
    let inside = |cx: i32, cy: i32| dept_id.map_or(true, |d| inside_dept(geometry, d, cx, cy));

    // 4 позиции со смещением offset от краёв объекта.
    let candidates: [(i32, i32); 4] = [
        (r_obj.x + r_obj.w + offset, r_obj.y),           // справа
        (r_obj.x - offset - ghost_w, r_obj.y),            // слева
        (r_obj.x, r_obj.y + r_obj.h + offset),            // снизу
        (r_obj.x, r_obj.y - offset - ghost_h),            // сверху
    ];
    for (cx, cy) in &candidates {
        if !inside(*cx, *cy) {
            continue;
        }
        let r_ghost = Rect::new(*cx, *cy, ghost_w, ghost_h);
        if rect_is_free(geometry, &r_ghost, exclude_id) {
            return Some((*cx, *cy));
        }
    }

    // Fallback: спиральный поиск от центра объекта, начиная снаружи от его границ.
    let cx0 = r_obj.x + r_obj.w / 2;
    let cy0 = r_obj.y + r_obj.h / 2;
    let min_r = (r_obj.w.max(r_obj.h) / 2) + offset;
    for r in min_r..=min_r + 12 {
        for dx in -r..=r {
            for dy in -r..=r {
                if dx.abs() != r && dy.abs() != r {
                    continue;
                }
                let cx = cx0 + dx;
                let cy = cy0 + dy;
                if !inside(cx, cy) {
                    continue;
                }
                let r_ghost = Rect::new(cx, cy, ghost_w, ghost_h);
                if rect_is_free(geometry, &r_ghost, exclude_id) {
                    return Some((cx, cy));
                }
            }
        }
    }
    None
}

/// Для отдела: свободная клетка рядом с содержащимися в нём процессами на расстоянии `offset`.
pub fn find_ghost_cell_in_dept(
    geometry: &Geometry,
    dept_id: &str,
    ghost_w: i32,
    ghost_h: i32,
    offset: i32,
    exclude_id: &str,
) -> Option<(i32, i32)> {
    let dept = geometry.objects.iter().find(|o| o.id == dept_id)?;
    let children: Vec<&crate::geometry::ObjectGeom> = geometry
        .objects
        .iter()
        .filter(|o| o.parent.as_deref() == Some(dept_id) && o.id != exclude_id)
        .collect();

    // Для каждого дочернего процесса пробуем 4 позиции со смещением offset от его краёв.
    for child in &children {
        let r_child = &child.rect;
        let candidates: [(i32, i32); 4] = [
            (r_child.x + r_child.w + offset, r_child.y),
            (r_child.x - offset - ghost_w, r_child.y),
            (r_child.x, r_child.y + r_child.h + offset),
            (r_child.x, r_child.y - offset - ghost_h),
        ];
        for (cx, cy) in candidates {
            let r_ghost = Rect::new(cx, cy, ghost_w, ghost_h);
            if rect_is_free(geometry, &r_ghost, exclude_id) {
                return Some((cx, cy));
            }
        }
    }

    // Fallback: спиральный поиск от центра отдела наружу.
    let cx0 = dept.rect.x + dept.rect.w / 2;
    let cy0 = dept.rect.y + dept.rect.h / 2;
    let min_r = offset;
    for r in min_r..=min_r + 24 {
        for dx in -r..=r {
            for dy in -r..=r {
                if dx.abs() != r && dy.abs() != r {
                    continue;
                }
                let cx = cx0 + dx;
                let cy = cy0 + dy;
                let r_ghost = Rect::new(cx, cy, ghost_w, ghost_h);
                if rect_is_free(geometry, &r_ghost, exclude_id) {
                    return Some((cx, cy));
                }
            }
        }
    }
    None
}

/// Вход для расчёта превью перетаскивания (призрачная копия + цель).
pub struct DragPreviewInput {
    /// Геометрия с наложенными предсказаниями.
    pub geometry: Geometry,
    /// ID перетаскиваемого объекта.
    pub dragged_id: String,
    /// Размер перетаскиваемого объекта (w, h в клетках).
    pub dragged_size: (i32, i32),
    /// Клетка под курсором (цель перетаскивания).
    pub cursor_cell: (i32, i32),
}

/// Результат расчёта превью перетаскивания.
#[derive(Clone, Debug, PartialEq)]
pub struct DragPreview {
    /// Призрачная клетка (свободное место рядом с целью), если найдена.
    pub ghost: Option<(i32, i32)>,
    /// Объект/отдел, над которым нависает курсор.
    pub over: Option<String>,
    /// Предсказанная позиция (призрак или клетка курсора).
    pub predicted: (i32, i32),
}

/// Рассчитать превью перетаскивания: призрачную копию, цель и предсказанную позицию.
pub fn compute_drag_preview(input: &DragPreviewInput) -> DragPreview {
    let DragPreviewInput {
        geometry,
        dragged_id,
        dragged_size,
        cursor_cell,
    } = input;

    let (w, h) = *dragged_size;
    let (cx, cy) = *cursor_cell;

    // Кто находится под курсором (исключая перетаскиваемый объект).
    let under_id = object_at_cell(geometry, cx, cy, dragged_id);

    let mut over: Option<String> = None;
    let mut ghost: Option<(i32, i32)> = None;

    if let Some(under_id) = under_id {
        if let Some(under_obj) = geometry.objects.iter().find(|o| o.id == under_id) {
            match under_obj.kind {
                NodeKind::Department => {
                    over = Some(under_id.to_owned());
                    let cursor_cell_tuple = (cx, cy);
                    let nearest_child = geometry.objects.iter()
                        .filter(|c| c.parent.as_deref() == Some(under_id) && c.id.as_str() != dragged_id.as_str())
                        .min_by_key(|c| {
                            let dx = c.rect.x + c.rect.w / 2 - cursor_cell_tuple.0;
                            let dy = c.rect.y + c.rect.h / 2 - cursor_cell_tuple.1;
                            dx.abs() + dy.abs()
                        });
                    if let Some(child) = nearest_child {
                        ghost = find_ghost_cell(geometry, child, w, h, 2, dragged_id, Some(under_id));
                    } else {
                        ghost = find_ghost_cell_in_dept(geometry, under_id, w, h, 2, dragged_id);
                    }
                }
                NodeKind::Process => {
                    let in_dept = under_obj.parent.clone().filter(|p| {
                        geometry.objects.iter().any(|oo| {
                            oo.id == *p && oo.kind == NodeKind::Department
                        })
                    });
                    if let Some(dept_id) = in_dept {
                        over = Some(dept_id.clone());
                        ghost = find_ghost_cell(geometry, under_obj, w, h, 2, dragged_id, Some(&dept_id));
                    } else {
                        over = Some(under_id.to_owned());
                        ghost = find_ghost_cell(geometry, under_obj, w, h, 2, dragged_id, None);
                    }
                }
                NodeKind::Container | NodeKind::Metric => {
                    // Контейнер и Показатель не принимают вложенность при драге.
                }
            }
        }
    }

    let predicted = ghost.unwrap_or(*cursor_cell);

    DragPreview { ghost, over, predicted }
}

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

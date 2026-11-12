//! Чистая доменная логика перетаскивания объектов на холсте.
//! Без egui, без окна, без сети. Только входное состояние и чистый результат.

use crate::geometry::Geometry;
use crate::layout::Rect;
use crate::types::{Direction, NodeKind};
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
    // Затем департаменты.
    for object in &geometry.objects {
        if object.kind != NodeKind::Department {
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
    None
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
                _ => {}
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
        Direction::Request => Direction::Output,
        Direction::Output => Direction::Request,
    };

    let entries = match target_dir {
        Direction::Request => &target.requests,
        Direction::Output => &target.outputs,
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
        Direction::Output => (
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

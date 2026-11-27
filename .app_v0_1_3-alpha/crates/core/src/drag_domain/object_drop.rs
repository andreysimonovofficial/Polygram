//! Точка входа для расчёта команд при дропе любого объекта (Отдел, Процесс, Контейнер, Показатель).

use crate::command::Command;
use crate::geometry::Geometry;
use crate::types::{ContainerDirection, NodeKind};
use crate::drag_domain::free_cell::target_object_at;

use super::dept_drop::compute_department_drop;
use super::process_drop::compute_process_drop;
use super::container_drop::compute_container_drop;
use super::metric_drop_object::compute_metric_drop_object;

/// Входное состояние для расчёта команд при дропе любого объекта.
pub struct ObjectDropContext {
    pub geometry: Geometry,
    pub dragged_id: String,
    pub start_cell: (i32, i32),
    pub drop_cell: (i32, i32),
    pub over_dept: Option<String>,
    pub dragged_size: (i32, i32),
    /// Направление контейнера при перетаскивании шапки контейнера из карточки процесса.
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
        let result = compute_container_drop(geometry, dragged_id, *dir, start_cell, drop_cell, over_dept_str, dragged_size, target);
        return ObjectDropResult {
            commands: result.commands,
            final_cell: result.final_cell,
            reparent_dept: result.reparent_dept,
        };
    }

    match dragged.kind {
        NodeKind::Department => {
            let result = compute_department_drop(geometry, dragged_id, start_cell, drop_cell, over_dept_str, dragged_size, target);
            ObjectDropResult {
                commands: result.commands,
                final_cell: result.final_cell,
                reparent_dept: result.reparent_dept,
            }
        }
        NodeKind::Process => {
            let result = compute_process_drop(geometry, dragged_id, start_cell, drop_cell, over_dept_str, dragged_size, target);
            ObjectDropResult {
                commands: result.commands,
                final_cell: result.final_cell,
                reparent_dept: result.reparent_dept,
            }
        }
        NodeKind::Container => {
            let dir = dragged.direction.unwrap_or(ContainerDirection::Request);
            let result = compute_container_drop(geometry, dragged_id, dir, start_cell, drop_cell, over_dept_str, dragged_size, target);
            ObjectDropResult {
                commands: result.commands,
                final_cell: result.final_cell,
                reparent_dept: result.reparent_dept,
            }
        }
        NodeKind::Metric => {
            let dir = dragged.direction.unwrap_or(ContainerDirection::Request);
            let result = compute_metric_drop_object(
                geometry,
                dragged_id,
                dragged_id,
                dir,
                start_cell,
                drop_cell,
                over_dept_str,
                dragged_size,
                target,
            );
            ObjectDropResult {
                commands: result.commands,
                final_cell: result.final_cell,
                reparent_dept: result.reparent_dept,
            }
        }
    }
}

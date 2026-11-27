//! Расчёт команд при дропе отдела.

use crate::command::Command;
use crate::geometry::{Geometry, ObjectGeom};
use crate::types::NodeKind;
use crate::drag_domain::free_cell::find_free_cell_in_dept_domain;

/// Результат расчёта команд при дропе отдела.
pub struct DepartmentDropResult {
    pub commands: Vec<Command>,
    pub final_cell: (i32, i32),
    pub reparent_dept: Option<String>,
}

/// Рассчитать команды при дропе отдела.
pub fn compute_department_drop(
    geometry: &Geometry,
    dragged_id: &str,
    start_cell: (i32, i32),
    drop_cell: (i32, i32),
    over_dept: Option<&str>,
    dragged_size: (i32, i32),
    target: Option<&ObjectGeom>,
) -> DepartmentDropResult {
    // Над отделом — репарентинг (модал обрабатывается на UI-слое).
    if let Some(dept_id) = over_dept {
        if geometry.objects.iter().any(|o| o.id == dept_id && o.kind == NodeKind::Department) {
            let final_cell = if target.map_or(false, |t| t.kind == NodeKind::Department) {
                find_free_cell_in_dept_domain(geometry, dept_id, dragged_size.0, dragged_size.1, dragged_id)
                    .unwrap_or(drop_cell)
            } else {
                drop_cell
            };
            return DepartmentDropResult {
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
        return DepartmentDropResult {
            commands: vec![Command::MoveObject { object: dragged_id.to_owned(), at: [drop_cell.0, drop_cell.1] }],
            final_cell: drop_cell,
            reparent_dept: None,
        };
    }
    DepartmentDropResult {
        commands: Vec::new(),
        final_cell: start_cell,
        reparent_dept: None,
    }
}

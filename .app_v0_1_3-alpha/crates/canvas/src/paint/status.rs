//! Ранжирование и агрегация статусов показателей.

use std::collections::HashMap;

use polygram_core::geometry::{EntryGeom, ObjectGeom};
use polygram_core::types::NodeKind;
use polygram_core::{DynamicState, StatusCode};

/// Ранг статуса (чем выше — тем выше приоритет показа / хуже состояние).
/// Ожидается > В процессе > В избытке > Обеспечен / Удовлетворён.
pub fn status_rank(s: StatusCode) -> u8 {
    match s {
        StatusCode::Ok => 0,
        StatusCode::Surplus => 1,
        StatusCode::InProgress => 2,
        StatusCode::Expected => 3,
    }
}

/// Худший (высокоприоритетнейший) статус среди записей.
pub fn worst_status(entries: &[EntryGeom], dynamic: &DynamicState) -> StatusCode {
    let mut worst: u8 = 0;
    for e in entries {
        let r = status_rank(dynamic.status[e.ix as usize]);
        if r > worst {
            worst = r;
        }
    }
    match worst {
        3 => StatusCode::Expected,
        2 => StatusCode::InProgress,
        1 => StatusCode::Surplus,
        _ => StatusCode::Ok,
    }
}

/// Агрегированный статус отдела: худший статус среди записей всех вложенных
/// процессов (прямых и через вложенные отделы). Пустой отдел -> Ok.
pub fn department_status(
    objects: &[ObjectGeom],
    dynamic: &DynamicState,
    dept: &ObjectGeom,
) -> StatusCode {
    let mut children: HashMap<&str, Vec<&ObjectGeom>> = HashMap::new();
    for o in objects {
        if let Some(p) = &o.parent {
            children.entry(p.as_str()).or_default().push(o);
        }
    }

    let mut worst: u8 = 0;
    let mut stack: Vec<&ObjectGeom> = children.get(dept.id.as_str()).cloned().unwrap_or_default();
    while let Some(cur) = stack.pop() {
        match cur.kind {
            NodeKind::Process => {
                for e in cur.requests.iter().chain(cur.results.iter()) {
                    let r = status_rank(dynamic.status[e.ix as usize]);
                    if r > worst {
                        worst = r;
                    }
                }
            }
            NodeKind::Department => {
                stack.extend(children.get(cur.id.as_str()).cloned().unwrap_or_default());
            }
            _ => {}
        }
    }

    match worst {
        3 => StatusCode::Expected,
        2 => StatusCode::InProgress,
        1 => StatusCode::Surplus,
        _ => StatusCode::Ok,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use polygram_core::layout::Rect;
    use polygram_core::types::MetricValue;

    fn dept(id: &str) -> ObjectGeom {
        ObjectGeom {
            ix: 0,
            id: id.into(),
            name: "Отдел".into(),
            kind: NodeKind::Department,
            rect: Rect::new(0, 0, 10, 10),
            direction: None,
            requests: vec![],
            results: vec![],
            parent: None,
        }
    }

    fn process(id: &str, parent: &str, entry_ixes: &[u32]) -> ObjectGeom {
        ObjectGeom {
            ix: 1,
            id: id.into(),
            name: "Процесс".into(),
            kind: NodeKind::Process,
            rect: Rect::new(0, 0, 5, 5),
            direction: None,
            requests: entry_ixes
                .iter()
                .map(|&ix| EntryGeom {
                    ix,
                    name: "r".into(),
                    value: MetricValue::Number(0.0),
                    supplied: MetricValue::Number(0.0),
                    unit: "".into(),
                })
                .collect(),
            results: vec![],
            parent: Some(parent.into()),
        }
    }

    fn dynamic(statuses: Vec<StatusCode>) -> DynamicState {
        DynamicState {
            status: statuses,
            ..Default::default()
        }
    }

    #[test]
    fn department_with_expected_process_is_expected() {
        let objects = vec![dept("d"), process("p1", "d", &[0])];
        let dept_obj = &objects[0];
        let dynamic = dynamic(vec![StatusCode::Expected]);
        assert_eq!(department_status(&objects, &dynamic, dept_obj), StatusCode::Expected);
    }

    #[test]
    fn department_expected_wins_over_surplus() {
        let objects = vec![dept("d"), process("p1", "d", &[0]), process("p2", "d", &[1])];
        let dept_obj = &objects[0];
        let dynamic = dynamic(vec![StatusCode::Expected, StatusCode::Surplus]);
        assert_eq!(department_status(&objects, &dynamic, dept_obj), StatusCode::Expected);
    }

    #[test]
    fn department_with_surplus_process_is_surplus() {
        let objects = vec![dept("d"), process("p1", "d", &[0])];
        let dept_obj = &objects[0];
        let dynamic = dynamic(vec![StatusCode::Surplus]);
        assert_eq!(department_status(&objects, &dynamic, dept_obj), StatusCode::Surplus);
    }

    #[test]
    fn empty_department_is_ok() {
        let objects = vec![dept("d")];
        let dept_obj = &objects[0];
        let dynamic = dynamic(vec![]);
        assert_eq!(department_status(&objects, &dynamic, dept_obj), StatusCode::Ok);
    }

    #[test]
    fn nested_department_process_counts() {
        let objects = vec![
            dept("d"),
            ObjectGeom {
                ix: 1,
                id: "sub".into(),
                name: "Подотдел".into(),
                kind: NodeKind::Department,
                rect: Rect::new(0, 0, 5, 5),
                direction: None,
                requests: vec![],
                results: vec![],
                parent: Some("d".into()),
            },
            process("p1", "sub", &[0]),
        ];
        let dept_obj = &objects[0];
        let dynamic = dynamic(vec![StatusCode::Expected]);
        assert_eq!(department_status(&objects, &dynamic, dept_obj), StatusCode::Expected);
    }
}

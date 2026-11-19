//! Тесты чистой доменной логики дроп объекта (compute_drop_outcome, compute_drag_preview).
use polygram_core::drag_domain::{
    compute_drag_preview, compute_drop_outcome, compute_metric_drop, DragPreviewInput, DropContext,
    DropOutcome, MetricDropContext,
};
use polygram_core::geometry::{Geometry, ObjectGeom};
use polygram_core::layout::Rect;
use polygram_core::types::{Direction, MetricKind, MetricValue, NodeKind};
use polygram_core::{Command, Engine, Scope};

fn obj(id: &str, x: i32, y: i32, w: i32, h: i32, kind: NodeKind) -> ObjectGeom {
    ObjectGeom {
        ix: 0,
        id: id.to_owned(),
        name: id.to_owned(),
        kind,
        rect: Rect::new(x, y, w, h),
        requests: vec![],
        results: vec![],
        parent: None,
    }
}

fn geometry_with(objects: Vec<ObjectGeom>) -> Geometry {
    Geometry {
        rev: 1,
        objects,
        edges: vec![],
    }
}

#[test]
fn drop_in_empty_cell_moves_object() {
    let geometry = geometry_with(vec![
        obj("a", 0, 0, 2, 2, NodeKind::Process),
    ]);
    let ctx = DropContext {
        geometry,
        dragged_id: "a".to_owned(),
        start_cell: (0, 0),
        drop_cell: (10, 10),
        over_dept: None,
        has_ghost: false,
        dragged_size: (2, 2),
    };
    assert_eq!(
        compute_drop_outcome(&ctx),
        DropOutcome::Move { at: (10, 10) }
    );
}

#[test]
fn drop_on_department_with_ghost_reparents_and_moves() {
    let geometry = geometry_with(vec![
        obj("a", 0, 0, 2, 2, NodeKind::Process),
        obj("d", 5, 5, 4, 4, NodeKind::Department),
    ]);
    let ctx = DropContext {
        geometry,
        dragged_id: "a".to_owned(),
        start_cell: (0, 0),
        drop_cell: (10, 5),
        over_dept: Some("d".to_owned()),
        has_ghost: true,
        dragged_size: (2, 2),
    };
    assert_eq!(
        compute_drop_outcome(&ctx),
        DropOutcome::ReparentAndMove { dept_id: "d".to_owned(), at: (10, 5) }
    );
}

#[test]
fn reorder_metric_entry_moves_within_requests() {
    let mut e = Engine::new();
    e.apply(
        Scope::Root,
        polygram_core::Command::AddObject {
            id: Some("a".into()),
            name: "a".into(),
            kind: polygram_core::types::NodeKind::Process,
            at: None,
            parent: None,
            author: None,
        },
    )
    .unwrap();
    e.apply(
        Scope::Root,
        polygram_core::Command::AddMetricDef {
            id: Some("md1".into()),
            name: "md1".into(),
            kind: MetricKind::Number,
            unit: "ед.".into(),
            api: None,
        },
    )
    .unwrap();
    // Добавляем три запроса: r1, r2, r3
    for name in ["r1", "r2", "r3"] {
        e.apply(
            Scope::Root,
            polygram_core::Command::AddMetricEntry {
                object: "a".into(),
                dir: Direction::Request,
                id: Some(name.into()),
                metric_def: "md1".into(),
                name: String::new(),
                value: MetricValue::Null,
                required: MetricValue::Null,
                max: None,
                fact: MetricValue::Null,
            },
        )
        .unwrap();
    }

    let geom = e.geometry();
    let obj = geom.objects.iter().find(|o| o.id == "a").unwrap();
    assert_eq!(obj.requests.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), vec!["r1", "r2", "r3"]);

    // Перемещаем r3 (индекс 2) на позицию 0
    e.apply(
        Scope::Root,
        polygram_core::Command::ReorderMetricEntry {
            object: "a".into(),
            dir: Direction::Request,
            from_ix: 2,
            to_ix: 0,
        },
    )
    .unwrap();

    let geom = e.geometry();
    let obj = geom.objects.iter().find(|o| o.id == "a").unwrap();
    assert_eq!(
        obj.requests.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
        vec!["r3", "r1", "r2"]
    );
}

#[test]
fn reorder_metric_entry_moves_within_outputs() {
    let mut e = Engine::new();
    e.apply(
        Scope::Root,
        polygram_core::Command::AddObject {
            id: Some("a".into()),
            name: "a".into(),
            kind: polygram_core::types::NodeKind::Process,
            at: None,
            parent: None,
            author: None,
        },
    )
    .unwrap();
    e.apply(
        Scope::Root,
        polygram_core::Command::AddMetricDef {
            id: Some("md1".into()),
            name: "md1".into(),
            kind: MetricKind::Number,
            unit: "ед.".into(),
            api: None,
        },
    )
    .unwrap();
    for name in ["o1", "o2", "o3"] {
        e.apply(
            Scope::Root,
            polygram_core::Command::AddMetricEntry {
                object: "a".into(),
                dir: Direction::Result,
                id: Some(name.into()),
                metric_def: "md1".into(),
                name: String::new(),
                value: MetricValue::Null,
                required: MetricValue::Null,
                max: None,
                fact: MetricValue::Null,
            },
        )
        .unwrap();
    }

    let geom = e.geometry();
    let obj = geom.objects.iter().find(|o| o.id == "a").unwrap();
    assert_eq!(obj.results.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), vec!["o1", "o2", "o3"]);

    // Перемещаем o1 (индекс 0) на позицию 2
    e.apply(
        Scope::Root,
        polygram_core::Command::ReorderMetricEntry {
            object: "a".into(),
                dir: Direction::Result,
            from_ix: 0,
            to_ix: 2,
        },
    )
    .unwrap();

    let geom = e.geometry();
    let obj = geom.objects.iter().find(|o| o.id == "a").unwrap();
    assert_eq!(
        obj.results.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
        vec!["o2", "o3", "o1"]
    );
}

#[test]
fn reorder_metric_entry_same_index_noop() {
    let mut e = Engine::new();
    e.apply(
        Scope::Root,
        polygram_core::Command::AddObject {
            id: Some("a".into()),
            name: "a".into(),
            kind: polygram_core::types::NodeKind::Process,
            at: None,
            parent: None,
            author: None,
        },
    )
    .unwrap();
    e.apply(
        Scope::Root,
        polygram_core::Command::AddMetricDef {
            id: Some("md1".into()),
            name: "md1".into(),
            kind: MetricKind::Number,
            unit: "ед.".into(),
            api: None,
        },
    )
    .unwrap();
    e.apply(
        Scope::Root,
        polygram_core::Command::AddMetricEntry {
            object: "a".into(),
            dir: Direction::Request,
            id: Some("r1".into()),
            metric_def: "md1".into(),
            name: String::new(),
            value: MetricValue::Null,
            required: MetricValue::Null,
            max: None,
            fact: MetricValue::Null,
        },
    )
    .unwrap();

    // Перемещение на ту же позицию — нетоп
    e.apply(
        Scope::Root,
        polygram_core::Command::ReorderMetricEntry {
            object: "a".into(),
            dir: Direction::Request,
            from_ix: 0,
            to_ix: 0,
        },
    )
    .unwrap();

    let geom = e.geometry();
    let obj = geom.objects.iter().find(|o| o.id == "a").unwrap();
    assert_eq!(obj.requests.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), vec!["r1"]);
}

#[test]
fn preview_over_empty_space_returns_cursor_cell() {
    let geometry = geometry_with(vec![
        obj("a", 0, 0, 2, 2, NodeKind::Process),
    ]);
    let input = DragPreviewInput {
        geometry,
        dragged_id: "a".to_owned(),
        dragged_size: (2, 2),
        cursor_cell: (10, 10),
    };
    let preview = compute_drag_preview(&input);
    assert_eq!(preview.ghost, None);
    assert_eq!(preview.over, None);
    assert_eq!(preview.predicted, (10, 10));
}

#[test]
fn preview_over_department_with_child_finds_ghost_near_child() {
    let geometry = geometry_with(vec![
        obj("a", 0, 0, 2, 2, NodeKind::Process),
        obj("d", 5, 5, 10, 10, NodeKind::Department),
        obj("c", 7, 7, 2, 2, NodeKind::Process),
    ]);
    let input = DragPreviewInput {
        geometry,
        dragged_id: "a".to_owned(),
        dragged_size: (2, 2),
        cursor_cell: (12, 12), // внутри отдела d, вне ребёнка c
    };
    let preview = compute_drag_preview(&input);
    assert_eq!(preview.over, Some("d".to_owned()));
    // Призрачная клетка найдена спиральным поиском (вне rect отдела), predicted = ghost
    assert_eq!(preview.predicted, preview.ghost.unwrap());
    assert!(preview.ghost.is_some());
}

#[test]
fn preview_over_department_without_children_finds_ghost_in_dept() {
    let geometry = geometry_with(vec![
        obj("a", 0, 0, 2, 2, NodeKind::Process),
        obj("d", 5, 5, 10, 10, NodeKind::Department),
    ]);
    let input = DragPreviewInput {
        geometry,
        dragged_id: "a".to_owned(),
        dragged_size: (2, 2),
        cursor_cell: (8, 8),
    };
    let preview = compute_drag_preview(&input);
    assert_eq!(preview.over, Some("d".to_owned()));
    assert!(preview.ghost.is_some());
}

#[test]
fn preview_over_process_returns_process_over() {
    let geometry = geometry_with(vec![
        obj("a", 0, 0, 2, 2, NodeKind::Process),
        obj("p", 5, 5, 4, 4, NodeKind::Process),
    ]);
    let input = DragPreviewInput {
        geometry,
        dragged_id: "a".to_owned(),
        dragged_size: (2, 2),
        cursor_cell: (6, 6), // над процессом p
    };
    let preview = compute_drag_preview(&input);
    assert_eq!(preview.over, Some("p".to_owned()));
    assert!(preview.ghost.is_some()); // свободная клетка рядом с p
}

#[test]
fn drop_on_department_without_ghost_cancels() {
    // drop_cell == start_cell означает, что призрачной клетки нет (место занято).
    let geometry = geometry_with(vec![
        obj("a", 0, 0, 2, 2, NodeKind::Process),
        obj("d", 5, 5, 10, 10, NodeKind::Department),
    ]);
    let ctx = DropContext {
        geometry,
        dragged_id: "a".to_owned(),
        start_cell: (0, 0),
        drop_cell: (0, 0),
        over_dept: Some("d".to_owned()),
        has_ghost: false,
        dragged_size: (2, 2),
    };
    assert_eq!(compute_drop_outcome(&ctx), DropOutcome::Cancel);
}

#[test]
fn drop_on_occupied_cell_cancels() {
    let geometry = geometry_with(vec![
        obj("a", 0, 0, 2, 2, NodeKind::Process),
        obj("b", 10, 10, 2, 2, NodeKind::Process),
    ]);
    let ctx = DropContext {
        geometry,
        dragged_id: "a".to_owned(),
        start_cell: (0, 0),
        drop_cell: (10, 10),
        over_dept: None,
        has_ghost: false,
        dragged_size: (2, 2),
    };
    assert_eq!(compute_drop_outcome(&ctx), DropOutcome::Cancel);
}

#[test]
fn drop_to_same_cell_cancels() {
    let geometry = geometry_with(vec![
        obj("a", 0, 0, 2, 2, NodeKind::Process),
    ]);
    let ctx = DropContext {
        geometry,
        dragged_id: "a".to_owned(),
        start_cell: (0, 0),
        drop_cell: (0, 0),
        over_dept: None,
        has_ghost: false,
        dragged_size: (2, 2),
    };
    assert_eq!(compute_drop_outcome(&ctx), DropOutcome::Cancel);
}

#[test]
fn drop_over_non_department_is_move() {
    let geometry = geometry_with(vec![
        obj("a", 0, 0, 2, 2, NodeKind::Process),
        obj("p", 5, 5, 2, 2, NodeKind::Process),
    ]);
    let ctx = DropContext {
        geometry,
        dragged_id: "a".to_owned(),
        start_cell: (0, 0),
        drop_cell: (10, 10),
        over_dept: Some("p".to_owned()),
        has_ghost: false,
        dragged_size: (2, 2),
    };
    assert_eq!(
        compute_drop_outcome(&ctx),
        DropOutcome::Move { at: (10, 10) }
    );
}

#[test]
fn metric_drop_adds_entry_and_connects() {
    // источник A с результатом "energy", цель B без показателей.
    let geometry = geometry_with(vec![
        obj("a", 0, 0, 2, 2, NodeKind::Process),
        obj("b", 5, 5, 2, 2, NodeKind::Process),
    ]);

    let ctx = MetricDropContext {
        geometry,
        source_object: "a".to_owned(),
        source_dir: Direction::Result,
        entry_name: "energy".to_owned(),
        target_id: "b".to_owned(),
    };

    let cmds = compute_metric_drop(&ctx);
    assert_eq!(cmds.len(), 2);

    // Первая команда — добавить запись в цель (противоположное направление).
    match &cmds[0] {
        Command::AddMetricEntry { object, dir, metric_def, name, .. } => {
            assert_eq!(object, "b");
            assert_eq!(*dir, Direction::Request); // источник Result -> цель Request
            assert_eq!(metric_def, "energy");
            assert_eq!(name, "energy");
        }
        other => panic!("expected AddMetricEntry, got {:?}", other),
    }

    // Вторая команда — связь из источника Result в цель Request.
    match &cmds[1] {
        Command::Connect { from_object, from_entry, to_object, to_entry, throughput } => {
            assert_eq!(from_object, "a");
            assert_eq!(from_entry, "energy");
            assert_eq!(to_object, "b");
            assert_eq!(to_entry, "energy");
            assert_eq!(*throughput, 100.0);
        }
        other => panic!("expected Connect, got {:?}", other),
    }
}

#[test]
fn metric_drop_same_object_noop() {
    let geometry = geometry_with(vec![
        obj("a", 0, 0, 2, 2, NodeKind::Process),
    ]);

    let ctx = MetricDropContext {
        geometry,
        source_object: "a".to_owned(),
        source_dir: Direction::Result,
        entry_name: "energy".to_owned(),
        target_id: "a".to_owned(),
    };

    let cmds = compute_metric_drop(&ctx);
    assert!(cmds.is_empty());
}

#[test]
fn metric_drop_target_not_process_noop() {
    let geometry = geometry_with(vec![
        obj("a", 0, 0, 2, 2, NodeKind::Process),
        obj("d", 5, 5, 4, 4, NodeKind::Department),
    ]);

    let ctx = MetricDropContext {
        geometry,
        source_object: "a".to_owned(),
        source_dir: Direction::Result,
        entry_name: "energy".to_owned(),
        target_id: "d".to_owned(),
    };

    let cmds = compute_metric_drop(&ctx);
    assert!(cmds.is_empty());
}

#[test]
fn metric_drop_target_already_has_entry_only_connects() {
    // цель уже имеет запись с таким именем в противоположном направлении — только Connect, без AddMetricEntry.
    let geometry = geometry_with(vec![
        obj("a", 0, 0, 2, 2, NodeKind::Process),
        ObjectGeom {
            ix: 0,
            id: "b".to_owned(),
            name: "b".to_owned(),
            kind: NodeKind::Process,
            rect: Rect::new(5, 5, 2, 2),
            requests: vec![polygram_core::geometry::EntryGeom {
                ix: 0,
                name: "energy".to_owned(),
                value: MetricValue::Null,
                supplied: MetricValue::Null,
                unit: String::new(),
            }],
            results: vec![],
            parent: None,
        },
    ]);

    let ctx = MetricDropContext {
        geometry,
        source_object: "a".to_owned(),
        source_dir: Direction::Result,
        entry_name: "energy".to_owned(),
        target_id: "b".to_owned(),
    };

    let cmds = compute_metric_drop(&ctx);
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        Command::Connect { from_object, .. } => assert_eq!(from_object, "a"),
        other => panic!("expected Connect, got {:?}", other),
    }
}

//! Тесты чистой доменной логики дроп объекта (compute_object_drop_commands, compute_metric_drop).
use polygram_core::drag_domain::{
    compute_metric_drop, MetricDropContext, ObjectDropContext,
};
use polygram_core::geometry::{EntryGeom, Geometry, ObjectGeom};
use polygram_core::layout::Rect;
use polygram_core::types::{ContainerDirection, Direction, MetricKind, MetricValue, NodeKind};
use polygram_core::{Command, Engine, Scope};

fn obj(id: &str, x: i32, y: i32, w: i32, h: i32, kind: NodeKind) -> ObjectGeom {
    ObjectGeom {
        ix: 0,
        id: id.to_owned(),
        name: id.to_owned(),
        kind,
        rect: Rect::new(x, y, w, h),
        direction: None,
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
            direction: None,
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
            direction: None,
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
            direction: None,
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
            direction: None,
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

// --- Tests for compute_object_drop_commands ---

fn dept_obj(id: &str, x: i32, y: i32, w: i32, h: i32) -> ObjectGeom {
    ObjectGeom {
        ix: 0,
        id: id.to_owned(),
        name: id.to_owned(),
        kind: NodeKind::Department,
        rect: Rect::new(x, y, w, h),
        direction: None,
        requests: vec![],
        results: vec![],
        parent: None,
    }
}

fn proc_obj(id: &str, x: i32, y: i32, w: i32, h: i32) -> ObjectGeom {
    ObjectGeom {
        ix: 0,
        id: id.to_owned(),
        name: id.to_owned(),
        kind: NodeKind::Process,
        rect: Rect::new(x, y, w, h),
        direction: None,
        requests: vec![],
        results: vec![],
        parent: None,
    }
}

fn container_obj(
    id: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    dir: ContainerDirection,
    entries: Vec<(&str, &str)>,
) -> ObjectGeom {
    ObjectGeom {
        ix: 0,
        id: id.to_owned(),
        name: id.to_owned(),
        kind: NodeKind::Container,
        rect: Rect::new(x, y, w, h),
        direction: Some(dir),
        requests: entries
            .iter()
            .filter(|(_, d)| *d == "req")
            .map(|(name, _)| EntryGeom {
                ix: 0,
                name: (*name).to_owned(),
                value: MetricValue::Null,
                supplied: MetricValue::Null,
                unit: String::new(),
            })
            .collect(),
        results: entries
            .iter()
            .filter(|(_, d)| *d == "res")
            .map(|(name, _)| EntryGeom {
                ix: 0,
                name: (*name).to_owned(),
                value: MetricValue::Null,
                supplied: MetricValue::Null,
                unit: String::new(),
            })
            .collect(),
        parent: None,
    }
}

fn metric_obj(id: &str, x: i32, y: i32, w: i32, h: i32, dir: ContainerDirection) -> ObjectGeom {
    ObjectGeom {
        ix: 0,
        id: id.to_owned(),
        name: id.to_owned(),
        kind: NodeKind::Metric,
        rect: Rect::new(x, y, w, h),
        direction: Some(dir),
        requests: vec![],
        results: vec![],
        parent: None,
    }
}

#[test]
fn object_drop_department_on_free_cell_moves() {
    let geometry = Geometry {
        rev: 1,
        objects: vec![dept_obj("d", 0, 0, 4, 4)],
        edges: vec![],
    };
    let result = polygram_core::drag_domain::compute_object_drop_commands(&ObjectDropContext {
        geometry,
        dragged_id: "d".to_owned(),
        start_cell: (0, 0),
        drop_cell: (10, 10),
        over_dept: None,
        dragged_size: (4, 4),
    });
    assert_eq!(result.commands.len(), 1);
    match &result.commands[0] {
        Command::MoveObject { object, at } => {
            assert_eq!(object, "d");
            assert_eq!(*at, [10, 10]);
        }
        other => panic!("expected MoveObject, got {:?}", other),
    }
}

#[test]
fn object_drop_department_on_department_reparents() {
    let geometry = Geometry {
        rev: 1,
        objects: vec![
            dept_obj("d1", 0, 0, 4, 4),
            dept_obj("d2", 10, 10, 4, 4),
        ],
        edges: vec![],
    };
    let result = polygram_core::drag_domain::compute_object_drop_commands(&ObjectDropContext {
        geometry,
        dragged_id: "d1".to_owned(),
        start_cell: (0, 0),
        drop_cell: (12, 12),
        over_dept: Some("d2".to_owned()),
        dragged_size: (4, 4),
    });
    assert_eq!(result.commands.len(), 2);
    assert!(result.reparent_dept.is_some());
    assert_eq!(result.reparent_dept.as_deref(), Some("d2"));
    match &result.commands[0] {
        Command::SetObjectParent { object, parent } => {
            assert_eq!(object, "d1");
            assert_eq!(parent.as_deref(), Some("d2"));
        }
        other => panic!("expected SetObjectParent, got {:?}", other),
    }
    match &result.commands[1] {
        Command::MoveObject { object, at } => {
            assert_eq!(object, "d1");
            assert_eq!(*at, [12, 12]);
        }
        other => panic!("expected MoveObject, got {:?}", other),
    }
}

#[test]
fn object_drop_process_on_free_cell_moves() {
    let geometry = Geometry {
        rev: 1,
        objects: vec![proc_obj("p", 0, 0, 2, 2)],
        edges: vec![],
    };
    let result = polygram_core::drag_domain::compute_object_drop_commands(&ObjectDropContext {
        geometry,
        dragged_id: "p".to_owned(),
        start_cell: (0, 0),
        drop_cell: (10, 10),
        over_dept: None,
        dragged_size: (2, 2),
    });
    assert_eq!(result.commands.len(), 1);
    match &result.commands[0] {
        Command::MoveObject { object, at } => {
            assert_eq!(object, "p");
            assert_eq!(*at, [10, 10]);
        }
        other => panic!("expected MoveObject, got {:?}", other),
    }
}

#[test]
fn object_drop_process_on_department_reparents() {
    let geometry = Geometry {
        rev: 1,
        objects: vec![
            proc_obj("p", 0, 0, 2, 2),
            dept_obj("d", 10, 10, 10, 10),
        ],
        edges: vec![],
    };
    let result = polygram_core::drag_domain::compute_object_drop_commands(&ObjectDropContext {
        geometry,
        dragged_id: "p".to_owned(),
        start_cell: (0, 0),
        drop_cell: (12, 12),
        over_dept: Some("d".to_owned()),
        dragged_size: (2, 2),
    });
    assert_eq!(result.commands.len(), 2);
    assert_eq!(result.reparent_dept.as_deref(), Some("d"));
    match &result.commands[0] {
        Command::SetObjectParent { object, parent } => {
            assert_eq!(object, "p");
            assert_eq!(parent.as_deref(), Some("d"));
        }
        other => panic!("expected SetObjectParent, got {:?}", other),
    }
}

#[test]
fn object_drop_container_request_on_free_cell_creates_process_and_copies_metrics() {
    use polygram_core::types::ContainerDirection;
    let geometry = Geometry {
        rev: 1,
        objects: vec![container_obj(
            "c",
            0,
            0,
            2,
            2,
            ContainerDirection::Request,
            vec![("m1", "req"), ("m2", "req")],
        )],
        edges: vec![],
    };
    let result = polygram_core::drag_domain::compute_object_drop_commands(&ObjectDropContext {
        geometry,
        dragged_id: "c".to_owned(),
        start_cell: (0, 0),
        drop_cell: (10, 10),
        over_dept: None,
        dragged_size: (2, 2),
    });
    // AddObject(Process) + 2x (AddMetricEntry + Connect) = 5 commands
    assert_eq!(result.commands.len(), 5);
    match &result.commands[0] {
        Command::AddObject { kind, name, at, .. } => {
            assert_eq!(*kind, NodeKind::Process);
            assert_eq!(name, "proc_from_c");
            assert_eq!(*at, Some([10, 10]));
        }
        other => panic!("expected AddObject, got {:?}", other),
    }
    // First metric entry: AddMetricEntry to process container (Result dir)
    match &result.commands[1] {
        Command::AddMetricEntry { object, dir, name, .. } => {
            assert_eq!(object, "proc_from_c");
            assert_eq!(*dir, Direction::Result);
            assert_eq!(name, "m1");
        }
        other => panic!("expected AddMetricEntry, got {:?}", other),
    }
    // First metric link: Connect from c.m1 to process.m1
    match &result.commands[2] {
        Command::Connect { from_object, from_entry, to_object, to_entry, .. } => {
            assert_eq!(from_object, "c");
            assert_eq!(from_entry, "m1");
            assert_eq!(to_object, "proc_from_c");
            assert_eq!(to_entry, "m1");
        }
        other => panic!("expected Connect, got {:?}", other),
    }
}

#[test]
fn object_drop_container_request_on_department_creates_process_and_reparents_dept() {
    use polygram_core::types::ContainerDirection;
    let geometry = Geometry {
        rev: 1,
        objects: vec![
            container_obj("c", 0, 0, 2, 2, ContainerDirection::Request, vec![("m1", "req")]),
            dept_obj("d", 10, 10, 10, 10),
        ],
        edges: vec![],
    };
    let result = polygram_core::drag_domain::compute_object_drop_commands(&ObjectDropContext {
        geometry,
        dragged_id: "c".to_owned(),
        start_cell: (0, 0),
        drop_cell: (12, 12),
        over_dept: Some("d".to_owned()),
        dragged_size: (2, 2),
    });
    assert_eq!(result.commands.len(), 4);
    assert_eq!(result.reparent_dept.as_deref(), Some("d"));
    // SetObjectParent: department → process (not process → container!)
    match &result.commands[1] {
        Command::SetObjectParent { object, parent } => {
            assert_eq!(object, "d");
            assert_eq!(parent.as_deref(), Some("proc_from_c"));
        }
        other => panic!("expected SetObjectParent, got {:?}", other),
    }
}

#[test]
fn object_drop_container_on_occupied_non_dept_cancels() {
    use polygram_core::types::ContainerDirection;
    let geometry = Geometry {
        rev: 1,
        objects: vec![
            container_obj("c", 0, 0, 2, 2, ContainerDirection::Request, vec![]),
            proc_obj("p", 10, 10, 2, 2),
        ],
        edges: vec![],
    };
    let result = polygram_core::drag_domain::compute_object_drop_commands(&ObjectDropContext {
        geometry,
        dragged_id: "c".to_owned(),
        start_cell: (0, 0),
        drop_cell: (10, 10),
        over_dept: None,
        dragged_size: (2, 2),
    });
    assert!(result.commands.is_empty());
}

#[test]
fn object_drop_metric_request_on_free_cell_creates_process_and_metric() {
    use polygram_core::types::ContainerDirection;
    let geometry = Geometry {
        rev: 1,
        objects: vec![metric_obj("m", 0, 0, 2, 2, ContainerDirection::Request)],
        edges: vec![],
    };
    let result = polygram_core::drag_domain::compute_object_drop_commands(&ObjectDropContext {
        geometry,
        dragged_id: "m".to_owned(),
        start_cell: (0, 0),
        drop_cell: (10, 10),
        over_dept: None,
        dragged_size: (2, 2),
    });
    // AddObject(Process) + AddMetricEntry + Connect = 3 commands
    assert_eq!(result.commands.len(), 3);
    match &result.commands[0] {
        Command::AddObject { kind, .. } => assert_eq!(*kind, NodeKind::Process),
        other => panic!("expected AddObject, got {:?}", other),
    }
    match &result.commands[1] {
        Command::AddMetricEntry { object, dir, name, .. } => {
            assert_eq!(object, "proc_from_m");
            assert_eq!(*dir, Direction::Result);
            assert_eq!(name, "m");
        }
        other => panic!("expected AddMetricEntry, got {:?}", other),
    }
    match &result.commands[2] {
        Command::Connect { from_object, from_entry, to_object, to_entry, .. } => {
            assert_eq!(from_object, "m");
            assert_eq!(from_entry, "m");
            assert_eq!(to_object, "proc_from_m");
            assert_eq!(to_entry, "m");
        }
        other => panic!("expected Connect, got {:?}", other),
    }
}

#[test]
fn object_drop_metric_request_on_department_creates_process_and_reparents() {
    use polygram_core::types::ContainerDirection;
    let geometry = Geometry {
        rev: 1,
        objects: vec![
            metric_obj("m", 0, 0, 2, 2, ContainerDirection::Request),
            dept_obj("d", 10, 10, 10, 10),
        ],
        edges: vec![],
    };
    let result = polygram_core::drag_domain::compute_object_drop_commands(&ObjectDropContext {
        geometry,
        dragged_id: "m".to_owned(),
        start_cell: (0, 0),
        drop_cell: (12, 12),
        over_dept: Some("d".to_owned()),
        dragged_size: (2, 2),
    });
    assert_eq!(result.commands.len(), 4);
    assert_eq!(result.reparent_dept.as_deref(), Some("d"));
    match &result.commands[1] {
        Command::SetObjectParent { object, parent } => {
            assert_eq!(object, "d");
            assert_eq!(parent.as_deref(), Some("proc_from_m"));
        }
        other => panic!("expected SetObjectParent, got {:?}", other),
    }
}

#[test]
fn object_drop_container_request_on_process_copies_metrics() {
    use polygram_core::types::ContainerDirection;
    let geometry = Geometry {
        rev: 1,
        objects: vec![
            container_obj("c", 0, 0, 2, 2, ContainerDirection::Request, vec![("m1", "req")]),
            proc_obj("p", 10, 10, 2, 2),
        ],
        edges: vec![],
    };
    let result = polygram_core::drag_domain::compute_object_drop_commands(&ObjectDropContext {
        geometry,
        dragged_id: "c".to_owned(),
        start_cell: (0, 0),
        drop_cell: (11, 11), // inside process rect
        over_dept: None,
        dragged_size: (2, 2),
    });
    // Copy metrics: AddMetricEntry + Connect = 2 commands (no new process)
    assert_eq!(result.commands.len(), 2);
    match &result.commands[0] {
        Command::AddMetricEntry { object, dir, name, .. } => {
            assert_eq!(object, "p");
            assert_eq!(*dir, Direction::Result);
            assert_eq!(name, "m1");
        }
        other => panic!("expected AddMetricEntry, got {:?}", other),
    }
    match &result.commands[1] {
        Command::Connect { from_object, from_entry, to_object, to_entry, .. } => {
            assert_eq!(from_object, "c");
            assert_eq!(from_entry, "m1");
            assert_eq!(to_object, "p");
            assert_eq!(to_entry, "m1");
        }
        other => panic!("expected Connect, got {:?}", other),
    }
}

#[test]
fn object_drop_container_on_occupied_container_cancels() {
    use polygram_core::types::ContainerDirection;
    let geometry = Geometry {
        rev: 1,
        objects: vec![
            container_obj("c", 0, 0, 2, 2, ContainerDirection::Request, vec![]),
            container_obj("c2", 10, 10, 2, 2, ContainerDirection::Result, vec![]),
        ],
        edges: vec![],
    };
    let result = polygram_core::drag_domain::compute_object_drop_commands(&ObjectDropContext {
        geometry,
        dragged_id: "c".to_owned(),
        start_cell: (0, 0),
        drop_cell: (11, 11),
        over_dept: None,
        dragged_size: (2, 2),
    });
    assert!(result.commands.is_empty());
}

#[test]
fn object_drop_same_cell_cancels() {
    let geometry = Geometry {
        rev: 1,
        objects: vec![dept_obj("d", 0, 0, 4, 4)],
        edges: vec![],
    };
    let result = polygram_core::drag_domain::compute_object_drop_commands(&ObjectDropContext {
        geometry,
        dragged_id: "d".to_owned(),
        start_cell: (0, 0),
        drop_cell: (0, 0),
        over_dept: None,
        dragged_size: (4, 4),
    });
    assert!(result.commands.is_empty());
}

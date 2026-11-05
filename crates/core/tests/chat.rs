//! Чат/лента событий: иерархическая видимость отдела.
//! Лента отдела показывает события собственных объектов и событий дочерних отделов.
use polygram_core::*;

fn dept(e: &mut Engine, id: &str, name: &str, parent: Option<&str>) {
    e.apply(
        Scope::Root,
        Command::AddObject {
            id: Some(id.into()),
            name: name.into(),
            kind: NodeKind::Department,
            at: None,
            parent: parent.map(|p| p.into()),
        },
    )
    .unwrap();
}

fn proc(e: &mut Engine, id: &str, parent: Option<&str>) {
    e.apply(
        Scope::Root,
        Command::AddObject {
            id: Some(id.into()),
            name: id.into(),
            kind: NodeKind::Process,
            at: None,
            parent: parent.map(|p| p.into()),
        },
    )
    .unwrap();
}

fn added_count(events: &[Event]) -> usize {
    events
        .iter()
        .filter(|ev| matches!(ev.kind, EventKind::ObjectAdded { .. }))
        .count()
}

#[test]
fn department_feed_shows_own_events() {
    let mut e = Engine::new();
    dept(&mut e, "d1", "Отдел 1", None);
    proc(&mut e, "p1", Some("d1"));

    let events = e.department_events("d1").unwrap();
    assert_eq!(added_count(&events), 2);
}

#[test]
fn department_feed_shows_child_events() {
    let mut e = Engine::new();
    dept(&mut e, "d1", "Отдел 1", None);
    dept(&mut e, "d2", "Отдел 2 (ребёнок)", Some("d1"));
    proc(&mut e, "p1", Some("d1"));
    proc(&mut e, "p2", Some("d2"));
    proc(&mut e, "p3", None); // вне дерева d1

    let events = e.department_events("d1").unwrap();
    // d1, d2, p1, p2 — внутри поддерева; p3 — снаружи
    assert_eq!(added_count(&events), 4);
}

#[test]
fn department_feed_shows_deeply_nested_child_events() {
    let mut e = Engine::new();
    dept(&mut e, "d1", "Отдел 1", None);
    dept(&mut e, "d2", "Отдел 2", Some("d1"));
    dept(&mut e, "d3", "Отдел 3", Some("d2"));
    proc(&mut e, "p1", Some("d3")); // deepest descendant

    let events = e.department_events("d1").unwrap();
    assert_eq!(added_count(&events), 4); // d1, d2, d3, p1
}

#[test]
fn department_feed_excludes_global_events() {
    let mut e = Engine::new();
    dept(&mut e, "d1", "Отдел 1", None);
    proc(&mut e, "p1", Some("d1"));
    e.apply(Scope::Root, Command::AutoLayout).unwrap(); // LayoutChanged — глобальное

    let events = e.department_events("d1").unwrap();
    assert!(!events.iter().any(|ev| matches!(ev.kind, EventKind::LayoutChanged)));
    // Отдельный отдел вне поддерева не попадает
    dept(&mut e, "d2", "Отдел 2", None);
    proc(&mut e, "p2", Some("d2"));
    let again = e.department_events("d1").unwrap();
    assert_eq!(added_count(&again), 2); // только d1, p1
}

#[test]
fn department_feed_not_found() {
    let e = Engine::new();
    assert!(e.department_events("нет_такого").is_err());
}

#[test]
fn department_feed_includes_metric_and_status_events() {
    let mut e = Engine::new();
    e.apply(
        Scope::Root,
        Command::AddMetricDef {
            id: Some("tovary".into()),
            name: "Товары".into(),
            kind: Default::default(),
            unit: "ед.".into(),
            api: None,
        },
    )
    .unwrap();
    dept(&mut e, "d1", "Отдел 1", None);
    proc(&mut e, "p1", Some("d1"));
    e.apply(
        Scope::Root,
        Command::AddMetricEntry {
            object: "p1".into(),
            dir: Direction::Output,
            id: Some("o1".into()),
            metric_def: "tovary".into(),
            name: String::new(),
            value: 10.0,
            required: 0.0,
            max: None,
            fact: 0.0,
        },
    )
    .unwrap();
    e.apply(
        Scope::Root,
        Command::SetValue {
            object: "p1".into(),
            entry: "o1".into(),
            value: 50.0,
        },
    )
    .unwrap();

    let events = e.department_events("d1").unwrap();
    let kinds: Vec<_> = events.iter().map(|ev| format!("{:?}", ev.kind)).collect();
    assert!(kinds.iter().any(|k| k.contains("MetricAdded")));
    assert!(kinds.iter().any(|k| k.contains("MetricChanged")));
}

#[test]
fn object_placed_inside_department_at_explicit_coords() {
    let mut e = Engine::new();
    dept(&mut e, "d1", "Отдел", None);
    // Отдел встанет авто-размещением
    let d_rect = e.layout().rect_of(e.hierarchy().find_object("d1").unwrap());
    // Явные координаты внутри отдела
    e.apply(
        Scope::Root,
        Command::AddObject {
            id: Some("p1".into()),
            name: "p1".into(),
            kind: NodeKind::Process,
            at: Some([d_rect.x + 2, d_rect.y + 3]),
            parent: Some("d1".into()),
        },
    )
    .unwrap();
    let p_rect = e.layout().rect_of(e.hierarchy().find_object("p1").unwrap());
    assert_eq!(p_rect.x, d_rect.x + 2);
    assert_eq!(p_rect.y, d_rect.y + 3);
}

#[test]
fn object_placed_near_when_cell_occupied_inside_department() {
    let mut e = Engine::new();
    dept(&mut e, "d1", "Отдел", None);
    let d_rect = e.layout().rect_of(e.hierarchy().find_object("d1").unwrap());
    let x = d_rect.x + 2;
    let y = d_rect.y + 3;
    // Первый процесс встанет на явно указанные координаты
    e.apply(
        Scope::Root,
        Command::AddObject {
            id: Some("p1".into()),
            name: "p1".into(),
            kind: NodeKind::Process,
            at: Some([x, y]),
            parent: Some("d1".into()),
        },
    )
    .unwrap();
    // Второй процесс с теми же координатами — должен встать в другое место внутри отдела
    e.apply(
        Scope::Root,
        Command::AddObject {
            id: Some("p2".into()),
            name: "p2".into(),
            kind: NodeKind::Process,
            at: Some([x, y]),
            parent: Some("d1".into()),
        },
    )
    .unwrap();
    let p2_rect = e.layout().rect_of(e.hierarchy().find_object("p2").unwrap());
    assert_ne!((p2_rect.x, p2_rect.y), (x, y));
    // p2 должен быть внутри отдека (ниже шапки)
    assert!(p2_rect.y >= d_rect.y + 2);
    assert!(p2_rect.y + p2_rect.h <= d_rect.y + d_rect.h);
    assert!(p2_rect.x >= d_rect.x);
    assert!(p2_rect.x + p2_rect.w <= d_rect.x + d_rect.w);
}

#[test]
fn object_outside_parent_not_allowed_to_overflow_silently() {
    let mut e = Engine::new();
    dept(&mut e, "d1", "Отдел", None);
    // Объект вне отдела без parent — встанет где указано
    e.apply(
        Scope::Root,
        Command::AddObject {
            id: Some("p1".into()),
            name: "p1".into(),
            kind: NodeKind::Process,
            at: Some([100, 100]),
            parent: None,
        },
    )
    .unwrap();
    let p_rect = e.layout().rect_of(e.hierarchy().find_object("p1").unwrap());
    assert_eq!(p_rect.x, 100);
    assert_eq!(p_rect.y, 100);
}

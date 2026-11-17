//! Сетка, запрет наложений, автораскладка и ортогональные линии.
mod common;
use common::*;
use polygram_core::*;

fn metric_def(e: &mut Engine, id: &str, name: &str, kind: MetricKind, unit: &str) {
    e.apply(
        Scope::Root,
        Command::AddMetricDef {
            id: Some(id.into()),
            name: name.into(),
            kind,
            unit: unit.into(),
            api: None,
        },
    )
    .unwrap();
}

#[test]
fn occupied_cells_are_rejected() {
    let mut e = Engine::new();
    object_at(&mut e, "a", [0, 0]).unwrap();
    assert_eq!(
        object_at(&mut e, "b", [0, 0]).unwrap_err().code(),
        "cell_occupied"
    );
    assert_eq!(
        object_at(&mut e, "b", [7, 1]).unwrap_err().code(),
        "cell_occupied"
    );
    object_at(&mut e, "b", [32, 0]).unwrap(); // с буфером 2 кванта справа — можно
    object_at(&mut e, "c", [0, 32]).unwrap(); // с буфером 2 кванта снизу — можно
    assert_eq!(e.stats().objects, 3);
    assert_no_overlaps(&e);
}

#[test]
fn auto_placement_never_overlaps() {
    let mut e = Engine::new();
    for k in 0..300 {
        object(&mut e, &format!("n{k}"));
    }
    assert_eq!(e.stats().objects, 300);
    assert_no_overlaps(&e);
}

#[test]
fn move_respects_occupancy_and_ghost_check() {
    let mut e = Engine::new();
    object_at(&mut e, "a", [0, 0]).unwrap(); // a: 0..30 x, 0..30 y
    object_at(&mut e, "b", [60, 0]).unwrap(); // b: 60..90 x, 0..30 y
    assert!(
        !e.can_place("a", [35, 0]).unwrap(),
        "35..65 задевает соседа в 60"
    );
    assert!(e.can_place("a", [5, 0]).unwrap(), "5..35 свободен");
    let r = e.apply(
        Scope::Root,
        Command::MoveObject {
            object: "a".into(),
            at: [35, 1],
        },
    );
    assert_eq!(r.unwrap_err().code(), "cell_occupied");
    e.apply(
        Scope::Root,
        Command::MoveObject {
            object: "a".into(),
            at: [5, 0],
        },
    )
    .unwrap();
    assert_eq!(e.object_view("a").unwrap().at, [5, 0]);
    // сам на себя можно (сдвиг на одну клетку внутри собственной площади)
    assert!(e.can_place("a", [4, 0]).unwrap());
    assert!(
        !e.can_place("a", [31, 0]).unwrap(),
        "31..61 задевает соседа в 60"
    );
}

#[test]
fn growing_card_relocates_instead_of_overlapping() {
    let mut e = Engine::new();
    metric_def(&mut e, "tovary", "Товары", MetricKind::Number, "ед.");
    object_at(&mut e, "a", [0, 0]).unwrap();
    object_at(&mut e, "b", [0, 32]).unwrap(); // с буфером 2 кванта под a
                                              // Добавляем максимум показателей — карточка остаётся 30×30 (минимум)
    for k in 0..MAX_METRICS_PER_SIDE {
        out(&mut e, "a", &format!("o{k}"), "tovary", MetricValue::Number(1.0), None);
    }
    assert_no_overlaps(&e);
    assert_eq!(e.object_view("b").unwrap().at, [0, 32], "соседа не двигаем");
    assert_eq!(
        e.object_view("a").unwrap().at,
        [0, 0],
        "карточка не переезжает (уже 30×30)"
    );
}

#[test]
fn auto_layout_is_overlap_free() {
    let mut rng = Rng(99);
    let mut e = Engine::new();
    metric_def(&mut e, "tovary", "Товары", MetricKind::Number, "ед.");
    for k in 0..300 {
        let id = format!("n{k}");
        object(&mut e, &id);
        let n_out = 1 + rng.below(3);
        let n_req = 1 + rng.below(3);
        for j in 0..n_out {
            out(&mut e, &id, &format!("o{j}"), "tovary", MetricValue::Number(10.0), None);
        }
        for j in 0..n_req {
            req(&mut e, &id, &format!("r{j}"), "tovary", MetricValue::Number(5.0), None);
        }
    }
    let mut connected = 0;
    for _ in 0..600 {
        let a = rng.below(300);
        let b = rng.below(300);
        let r = e.apply(
            Scope::Root,
            Command::Connect {
                from_object: format!("n{a}"),
                from_entry: format!("o{}", rng.below(2)),
                to_object: format!("n{b}"),
                to_entry: format!("r{}", rng.below(2)),
                throughput: 100.0,
            },
        );
        if r.is_ok() {
            connected += 1;
        }
    }
    assert!(connected > 100);
    e.apply(Scope::Root, Command::AutoLayout).unwrap();
    assert_no_overlaps(&e);
    let g = e.geometry();
    assert_eq!(g.edges.len(), e.stats().edges);
    assert_eq!(e.dynamic_state(), e.recomputed_state());
}

#[test]
fn auto_layout_puts_sources_left_of_consumers() {
    let mut e = Engine::new();
    metric_def(&mut e, "tovary", "Товары", MetricKind::Number, "ед.");
    for id in ["a", "b", "c"] {
        object(&mut e, id);
        out(&mut e, id, "o1", "tovary", MetricValue::Number(1.0), None);
        req(&mut e, id, "r1", "tovary", MetricValue::Number(1.0), None);
    }
    connect(&mut e, "a", "o1", "b", "r1");
    connect(&mut e, "b", "o1", "c", "r1");
    e.apply(Scope::Root, Command::AutoLayout).unwrap();
    let x = |id: &str| e.object_view(id).unwrap().at[0];
    assert!(x("a") < x("b") && x("b") < x("c"));
}

#[test]
fn events_are_ordered_and_gaps_detected() {
    let mut e = Engine::new();
    metric_def(&mut e, "tovary", "Товары", MetricKind::Number, "ед.");
    object(&mut e, "a");
    let r1 = e.rev();
    out(&mut e, "a", "o1", "tovary", MetricValue::Number(1.0), None);
    let evs = e.events_since(r1).unwrap();
    assert!(!evs.is_empty() && evs.iter().all(|x| x.rev > r1));
    assert!(evs.windows(2).all(|w| w[0].rev <= w[1].rev));
    let log = polygram_core::event::EventLog::new(3);
    let mut log = log;
    for r in 1..=5 {
        log.push(Event {
            rev: r,
            kind: EventKind::LayoutChanged,
        });
    }
    assert_eq!(log.since(0).unwrap_err().oldest_available, 3);
    assert_eq!(log.since(2).unwrap().len(), 3);
}

#[test]
fn auto_layout_leaves_room_for_two_cell_line_stubs() {
    // Линия выходит из карточки и входит в карточку минимум на 2 клетки каждая;
    // чтобы обе вертикали совпали, между колонками нужно минимум 4 клетки.
    let mut e = Engine::new();
    metric_def(&mut e, "tovary", "Товары", MetricKind::Number, "ед.");
    for id in ["a", "b"] {
        object(&mut e, id);
        out(&mut e, id, "o1", "tovary", MetricValue::Number(1.0), None);
        req(&mut e, id, "r1", "tovary", MetricValue::Number(1.0), None);
    }
    connect(&mut e, "a", "o1", "b", "r1");
    e.apply(Scope::Root, Command::AutoLayout).unwrap();
    let g = e.geometry();
    let ra = g.objects.iter().find(|o| o.id == "a").unwrap().rect;
    let rb = g.objects.iter().find(|o| o.id == "b").unwrap().rect;
    assert!(rb.x - (ra.x + ra.w) >= 4, "зазор между колонками: {ra:?} -> {rb:?}");
}

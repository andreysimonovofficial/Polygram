//! Слой линий на настоящей геометрии ядра: автораскладка → маршруты всех связей → DoD.

use super::layer::{terminal, EdgeLayer};
use super::model::Quality;
use super::tests::assert_dod;
use polygram_core::*;
use std::time::Instant;

pub(crate) fn engine() -> Engine {
    let mut e = Engine::new();
    e.apply(
        Scope::Root,
        Command::AddMetricDef {
            id: Some("m".into()),
            name: "m".into(),
            kind: MetricKind::Number,
            unit: "ед.".into(),
            api: None,
        },
    )
    .unwrap();
    for i in 0..12 {
        let id = format!("n{i}");
        e.apply(
            Scope::Root,
            Command::AddObject {
                id: Some(id.clone()),
                name: id.clone(),
                kind: NodeKind::Process,
                at: None,
                parent: None,
                author: None,
            },
        )
        .unwrap();
        for (dir, prefix) in [(Direction::Output, "o"), (Direction::Request, "r")] {
            for k in 0..2 {
                e.apply(
                    Scope::Root,
                    Command::AddMetricEntry {
                        object: id.clone(),
                        dir,
                        id: Some(format!("{prefix}{k}")),
                        metric_def: "m".into(),
                        name: String::new(),
                        value: if dir == Direction::Output {
                            MetricValue::Number(5.0)
                        } else {
                            MetricValue::Null
                        },
                        required: if dir == Direction::Request {
                            MetricValue::Number(1.0)
                        } else {
                            MetricValue::Null
                        },
                        max: None,
                        fact: MetricValue::Null,
                    },
                )
                .unwrap();
            }
        }
    }
    let link = |e: &mut Engine, a: usize, ao: &str, b: usize, br: &str| {
        let _ = e.apply(
            Scope::Root,
            Command::Connect {
                from_object: format!("n{a}"),
                from_entry: ao.into(),
                to_object: format!("n{b}"),
                to_entry: br.into(),
                throughput: 100.0,
            },
        );
    };
    for i in 0..11 {
        link(&mut e, i, "o0", i + 1, "r0");
    }
    for i in 0..9 {
        link(&mut e, i, "o1", i + 3, "r1"); // через несколько колонок: обходят чужие карточки
    }
    link(&mut e, 11, "o1", 0, "r1"); // обратная связь
    e.apply(Scope::Root, Command::AutoLayout).unwrap();
    e
}

#[test]
fn every_link_of_an_autolayout_graph_meets_dod() {
    let e = engine();
    let g = e.geometry();
    assert!(g.edges.len() > 15);
    let mut layer = EdgeLayer::default();
    layer.sync(&g, 16.0);
    let cards: Vec<_> = g.objects.iter().map(|o| o.rect).collect();

    let t = Instant::now();
    let mut quality = [0usize; 4];
    for edge in &g.edges {
        let ends = layer.ends(&g, edge).expect("концы связи");
        let src = terminal(ends.from, EntryIx(edge.from), true, 16.0).unwrap();
        let dst = terminal(ends.to, EntryIx(edge.to), false, 16.0).unwrap();
        let r = layer.route(&g, edge).expect("маршрут").clone();
        quality[r.quality as usize] += 1;
        assert_dod(&r, &src, &dst, &cards);
    }
    eprintln!(
        "{} связей за {:?}, качество {:?}",
        g.edges.len(),
        t.elapsed(),
        quality
    );
    assert_eq!(
        quality[Quality::Fallback as usize],
        0,
        "для автораскладки пути находятся всегда"
    );
}

#[test]
fn routes_are_cached_until_cards_move() {
    let e = engine();
    let mut g = e.geometry();
    let mut layer = EdgeLayer::default();
    layer.sync(&g, 16.0);
    let edge = g.edges[0].clone();
    let before = layer.route(&g, &edge).unwrap().clone();

    // Тот же снимок: кэш сохраняется, маршрут тот же.
    layer.sync(&g, 16.0);
    assert_eq!(layer.route(&g, &edge).unwrap(), &before);

    // Карточка приёмника сдвинулась: маршрут пересчитывается.
    let to = layer.ends(&g, &edge).unwrap().to.id.clone();
    g.objects.iter_mut().find(|o| o.id == to).unwrap().rect.y += 12;
    layer.sync(&g, 16.0);
    let after = layer.route(&g, &edge).unwrap().clone();
    assert_ne!(after, before);
}

#[test]
fn department_frame_is_not_an_obstacle() {
    let e = engine();
    let mut g = e.geometry();
    let mut layer = EdgeLayer::default();
    // Огромный отдел-рамка поверх всего: линии всё равно строятся.
    let mut frame = g.objects[0].clone();
    frame.kind = NodeKind::Department;
    frame.id = "dept".into();
    frame.requests.clear();
    frame.outputs.clear();
    frame.rect = polygram_core::Rect::new(-5, -5, 2000, 2000);
    g.objects.push(frame);
    layer.sync(&g, 16.0);
    let edge = g.edges[0].clone();
    let r = layer.route(&g, &edge).unwrap();
    assert_eq!(r.quality, Quality::Clear);
}

#[test]
fn moving_a_far_card_does_not_recompute_other_links() {
    let e = engine();
    let mut g = e.geometry();
    let mut layer = EdgeLayer::default();
    layer.sync(&g, 16.0);
    // Первая связь: между соседними карточками левой части графа.
    let edge = g.edges[0].clone();
    let ends = layer.ends(&g, &edge).unwrap();
    let (a, b) = (ends.from.id.clone(), ends.to.id.clone());
    layer.route(&g, &edge).unwrap();
    let base = layer.computed;

    // Двигаем карточку на другом конце графа: линия не пересчитывается.
    let far = g
        .objects
        .iter()
        .filter(|o| o.id != a && o.id != b)
        .max_by_key(|o| o.rect.x)
        .unwrap()
        .clone();
    assert!(
        far.rect.x > 300,
        "карточка достаточно далеко от первой связи"
    );
    let far = far.id;
    g.objects.iter_mut().find(|o| o.id == far).unwrap().rect.y += 7;
    layer.sync(&g, 16.0);
    layer.route(&g, &edge).unwrap();
    assert_eq!(layer.computed, base, "далёкая карточка не влияет на линию");

    // Двигаем свою карточку: линия пересчитывается.
    g.objects.iter_mut().find(|o| o.id == b).unwrap().rect.y += 7;
    layer.sync(&g, 16.0);
    layer.route(&g, &edge).unwrap();
    assert_eq!(layer.computed, base + 1);
}

//! Главная проверка корректности: после любых операций инкрементальный расчёт
//! обязан совпадать с полным пересчётом с нуля.
mod common;
use common::*;
use polygram_core::*;

fn random_metric(e: &Engine, id: &str, rng: &mut Rng, dir: Option<Direction>) -> Option<String> {
    let v = e.object_view(id)?;
    let list: Vec<&MetricView> = match dir {
        Some(Direction::Request) => v.requests.iter().collect(),
        Some(Direction::Output) => v.outputs.iter().collect(),
        None => v.requests.iter().chain(v.outputs.iter()).collect(),
    };
    if list.is_empty() {
        None
    } else {
        Some(list[rng.below(list.len())].id.clone())
    }
}

#[test]
fn incremental_equals_full_recompute_under_random_operations() {
    for seed in [1u64, 7, 42, 2026] {
        let mut rng = Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
        let mut e = Engine::new();
        metric_def(&mut e, "tovary", "Товары", MetricKind::Number, "ед.");
        let mut ids: Vec<String> = Vec::new();
        for k in 0..30 {
            let id = format!("n{k}");
            object(&mut e, &id);
            out(&mut e, &id, "o1", "tovary", rng.unit() * 100.0, None);
            out(&mut e, &id, "o2", "tovary", rng.unit() * 100.0, Some(50.0));
            req(&mut e, &id, "r1", "tovary", rng.unit() * 80.0, None);
            req(&mut e, &id, "r2", "tovary", rng.unit() * 80.0, Some(40.0));
            ids.push(id);
        }
        let mut counter = 100;
        for step in 0..3000 {
            let a = ids[rng.below(ids.len())].clone();
            let b = ids[rng.below(ids.len())].clone();
            let cmd = match rng.below(12) {
                0 | 1 | 2 => random_metric(&e, &a, &mut rng, Some(Direction::Output)).map(|m| Command::SetValue {
                    object: a.clone(),
                    entry: m,
                    value: (rng.unit() * 120.0 * 100.0).round() / 100.0,
                }),
                3 => random_metric(&e, &a, &mut rng, Some(Direction::Request)).map(|m| Command::SetRequired {
                    object: a.clone(),
                    entry: m,
                    required: (rng.unit() * 90.0).round(),
                }),
                4 => random_metric(&e, &a, &mut rng, None).map(|m| Command::SetMax {
                    object: a.clone(),
                    entry: m,
                    max: if rng.below(3) == 0 { None } else { Some((rng.unit() * 70.0).round()) },
                }),
                5 | 6 | 7 => {
                    let m1 = random_metric(&e, &a, &mut rng, Some(Direction::Output));
                    let m2 = random_metric(&e, &b, &mut rng, Some(Direction::Request));
                    match (m1, m2) {
                        (Some(m1), Some(m2)) => Some(Command::Connect {
                            from_object: a.clone(),
                            from_entry: m1,
                            to_object: b.clone(),
                            to_entry: m2,
                            throughput: 100.0,
                        }),
                        _ => None,
                    }
                }
                8 => {
                    let m1 = random_metric(&e, &a, &mut rng, Some(Direction::Output));
                    let m2 = random_metric(&e, &b, &mut rng, Some(Direction::Request));
                    match (m1, m2) {
                        (Some(m1), Some(m2)) => Some(Command::Disconnect {
                            from_object: a.clone(),
                            from_entry: m1,
                            to_object: b.clone(),
                            to_entry: m2,
                        }),
                        _ => None,
                    }
                }
                9 => random_metric(&e, &a, &mut rng, None).map(|m| Command::RemoveMetricEntry { object: a.clone(), entry: m }),
                10 => Some(Command::AddMetricEntry {
                    object: a.clone(),
                    dir: if rng.below(2) == 0 { Direction::Request } else { Direction::Output },
                    id: None,
                    metric_def: "tovary".into(),
                    name: "новый".into(),
                    value: (rng.unit() * 50.0).round(),
                    required: (rng.unit() * 50.0).round(),
                    max: None,
                    fact: 0.0,
                }),
                _ => {
                    if rng.below(2) == 0 && ids.len() > 10 {
                        let pos = rng.below(ids.len());
                        let gone = ids.remove(pos);
                        Some(Command::RemoveObject { object: gone })
                    } else {
                        counter += 1;
                        let id = format!("x{counter}");
                        ids.push(id.clone());
                        Some(Command::AddObject { id: Some(id), name: "новый".into(), kind: NodeKind::Process, at: None, parent: None })
                    }
                }
            };
            if let Some(cmd) = cmd {
                let _ = e.apply(Scope::Root, cmd); // часть команд законно отклоняется (дубликаты, лимиты)
            }
            if step % 25 == 0 {
                assert_eq!(e.dynamic_state(), e.recomputed_state(), "расхождение на шаге {step}, seed {seed}");
            }
        }
        assert_eq!(e.dynamic_state(), e.recomputed_state(), "итоговое расхождение, seed {seed}");
        assert_no_overlaps(&e);
        for edge in e.geometry().edges {
            assert_orthogonal(&edge.points);
        }
    }
}

#[test]
fn removing_nodes_updates_neighbours() {
    let mut e = Engine::new();
    metric_def(&mut e, "tovary", "Товары", MetricKind::Number, "ед.");
    object(&mut e, "p");
    out(&mut e, "p", "o1", "tovary", 100.0, None);
    object(&mut e, "a");
    req(&mut e, "a", "r1", "tovary", 100.0, None);
    connect(&mut e, "p", "o1", "a", "r1");
    assert_eq!(status(&e, "a", "r1"), StatusCode::Ok);
    e.apply(Scope::Root, Command::RemoveObject { object: "p".into() }).unwrap();
    assert_eq!(status(&e, "a", "r1"), StatusCode::Unconnected);
    assert_eq!(e.stats().edges, 0);
    assert_eq!(e.dynamic_state(), e.recomputed_state());
}

#[test]
fn slots_are_reused_without_stale_state() {
    let mut e = Engine::new();
    metric_def(&mut e, "tovary", "Товары", MetricKind::Number, "ед.");
    object(&mut e, "p");
    out(&mut e, "p", "o1", "tovary", 100.0, None);
    object(&mut e, "a");
    req(&mut e, "a", "r1", "tovary", 100.0, None);
    connect(&mut e, "p", "o1", "a", "r1");
    e.apply(Scope::Root, Command::RemoveObject { object: "a".into() }).unwrap();
    object(&mut e, "b");
    req(&mut e, "b", "r1", "tovary", 10.0, None);
    assert_eq!(status(&e, "b", "r1"), StatusCode::Unconnected, "в новом слоте не должно остаться чужого состояния");
    assert_eq!(e.dynamic_state(), e.recomputed_state());
}

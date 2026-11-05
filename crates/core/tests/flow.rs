//! Правила расчёта потоков и статусов: перенесены из MVP, проверяются на тех же примерах.
mod common;
use common::*;
use polygram_core::*;

fn producer_with_two_consumers(value: f64) -> Engine {
    let mut e = Engine::new();
    metric_def(&mut e, "tovary", "Товары", MetricKind::Number, "ед.");
    object(&mut e, "p");
    out(&mut e, "p", "o1", "tovary", value, None);
    object(&mut e, "a");
    req(&mut e, "a", "r1", "tovary", 60.0, None);
    object(&mut e, "b");
    req(&mut e, "b", "r1", "tovary", 40.0, None);
    connect(&mut e, "p", "o1", "a", "r1");
    connect(&mut e, "p", "o1", "b", "r1");
    e
}

#[test]
fn supply_is_shared_proportionally() {
    let e = producer_with_two_consumers(50.0);
    assert!((value_of(&e, "a", "r1") - 50.0).abs() < 1e-9);
    assert!((value_of(&e, "b", "r1") - 40.0).abs() < 1e-9);
    assert_eq!(status(&e, "a", "r1"), StatusCode::Deficit);
    assert_eq!(status(&e, "b", "r1"), StatusCode::Ok);
    assert_eq!(status(&e, "p", "o1"), StatusCode::Deficit);
}

#[test]
fn exact_and_surplus_supply() {
    let mut e = producer_with_two_consumers(50.0);
    set_value(&mut e, "p", "o1", 100.0);
    assert_eq!(status(&e, "a", "r1"), StatusCode::Ok);
    assert_eq!(status(&e, "b", "r1"), StatusCode::Ok);
    assert_eq!(status(&e, "p", "o1"), StatusCode::Ok);
    set_value(&mut e, "p", "o1", 150.0);
    assert_eq!(status(&e, "p", "o1"), StatusCode::Surplus);
    assert!((value_of(&e, "a", "r1") - 60.0).abs() < 1e-9, "лишнее не раздаётся сверх требования");
}

#[test]
fn zero_supply_is_deficit() {
    let e = producer_with_two_consumers(0.0);
    assert_eq!(status(&e, "a", "r1"), StatusCode::Deficit);
    assert_eq!(status(&e, "b", "r1"), StatusCode::Deficit);
    assert_eq!(status(&e, "p", "o1"), StatusCode::Deficit);
}

#[test]
fn unconnected_metrics() {
    let mut e = Engine::new();
    metric_def(&mut e, "tovary", "Товары", MetricKind::Number, "ед.");
    object(&mut e, "x");
    out(&mut e, "x", "o1", "tovary", 5.0, None);
    req(&mut e, "x", "r1", "tovary", 5.0, None);
    assert_eq!(status(&e, "x", "o1"), StatusCode::Unconnected);
    assert_eq!(status(&e, "x", "r1"), StatusCode::Unconnected);
}

#[test]
fn output_limit_is_sum_of_target_limits() {
    let mut e = Engine::new();
    metric_def(&mut e, "tovary", "Товары", MetricKind::Number, "ед.");
    object(&mut e, "p");
    out(&mut e, "p", "o1", "tovary", 60.0, Some(1000.0));
    object(&mut e, "a");
    req(&mut e, "a", "r1", "tovary", 60.0, Some(30.0));
    object(&mut e, "b");
    req(&mut e, "b", "r1", "tovary", 40.0, Some(20.0));
    connect(&mut e, "p", "o1", "a", "r1");
    connect(&mut e, "p", "o1", "b", "r1");
    // спрос 100 > значение 60 → дефицит производителя
    assert_eq!(status(&e, "p", "o1"), StatusCode::Deficit);
    let v = e.object_view("p").unwrap();
    assert_eq!(v.outputs[0].max, Some(1000.0));
}

#[test]
fn own_limit_applies_when_a_target_has_no_limit() {
    let mut e = Engine::new();
    metric_def(&mut e, "tovary", "Товары", MetricKind::Number, "ед.");
    object(&mut e, "p");
    out(&mut e, "p", "o1", "tovary", 90.0, Some(80.0));
    object(&mut e, "a");
    req(&mut e, "a", "r1", "tovary", 60.0, Some(30.0));
    object(&mut e, "b");
    req(&mut e, "b", "r1", "tovary", 40.0, None);
    connect(&mut e, "p", "o1", "a", "r1");
    connect(&mut e, "p", "o1", "b", "r1");
    // спрос 100 > значение 90 → дефицит
    assert_eq!(status(&e, "p", "o1"), StatusCode::Deficit);
    assert_eq!(e.object_view("p").unwrap().outputs[0].max, Some(80.0));
}

#[test]
fn requirement_change_updates_all_sources() {
    let mut e = producer_with_two_consumers(100.0);
    assert_eq!(status(&e, "p", "o1"), StatusCode::Ok);
    set_required(&mut e, "a", "r1", 160.0);
    // спрос 200 > значение 100 → дефицит производителя
    assert_eq!(status(&e, "p", "o1"), StatusCode::Deficit);
    assert!((value_of(&e, "a", "r1") - 100.0).abs() < 1e-9);
    assert!((value_of(&e, "b", "r1") - 40.0).abs() < 1e-9);
}

#[test]
fn bad_input_is_rejected() {
    let mut e = producer_with_two_consumers(10.0);
    let r = e.apply(Scope::Root, Command::SetValue { object: "p".into(), entry: "o1".into(), value: f64::NAN });
    assert_eq!(r.unwrap_err().code(), "invalid_number");
    let r = e.apply(Scope::Root, Command::SetRequired { object: "p".into(), entry: "o1".into(), required: 1.0 });
    assert_eq!(r.unwrap_err().code(), "wrong_direction");
    let r = e.apply(Scope::Root, Command::SetValue { object: "нет".into(), entry: "o1".into(), value: 1.0 });
    assert_eq!(r.unwrap_err().code(), "object_not_found");
}

#[test]
fn invalid_connections_are_rejected() {
    let mut e = producer_with_two_consumers(10.0);
    let dup = e.apply(
        Scope::Root,
        Command::Connect { from_object: "p".into(), from_entry: "o1".into(), to_object: "a".into(), to_entry: "r1".into(), throughput: 100.0 },
    );
    assert_eq!(dup.unwrap_err().code(), "edge_exists");
    let reversed = e.apply(
        Scope::Root,
        Command::Connect { from_object: "a".into(), from_entry: "r1".into(), to_object: "p".into(), to_entry: "o1".into(), throughput: 100.0 },
    );
    assert_eq!(reversed.unwrap_err().code(), "invalid_connection");
    object(&mut e, "q");
    out(&mut e, "q", "o1", "tovary", 1.0, None);
    req(&mut e, "q", "r1", "tovary", 1.0, None);
    let selfloop = e.apply(
        Scope::Root,
        Command::Connect { from_object: "q".into(), from_entry: "o1".into(), to_object: "q".into(), to_entry: "r1".into(), throughput: 100.0 },
    );
    assert_eq!(selfloop.unwrap_err().code(), "invalid_connection");
}

#[test]
fn metric_limit_per_side() {
    let mut e = Engine::new();
    metric_def(&mut e, "tovary", "Товары", MetricKind::Number, "ед.");
    object(&mut e, "x");
    for k in 0..MAX_METRICS_PER_SIDE {
        out(&mut e, "x", &format!("o{k}"), "tovary", 0.0, None);
    }
    let r = e.apply(
        Scope::Root,
        Command::AddMetricEntry {
            object: "x".into(),
            dir: Direction::Output,
            id: None,
            metric_def: "tovary".into(),
            name: "лишний".into(),
            value: 0.0,
            required: 0.0,
            max: None,
            fact: 0.0,
        },
    );
    assert_eq!(r.unwrap_err().code(), "too_many_metrics");
}

#[test]
fn bottlenecks_are_ranked_by_shortfall() {
    let mut e = Engine::new();
    metric_def(&mut e, "tovary", "Товары", MetricKind::Number, "ед.");
    for (id, value) in [("p", 50.0), ("q", 90.0)] {
        object(&mut e, id);
        out(&mut e, id, "o1", "tovary", value, None);
    }
    object(&mut e, "c");
    req(&mut e, "c", "a", "tovary", 100.0, None);
    req(&mut e, "c", "b", "tovary", 100.0, None);
    connect(&mut e, "p", "o1", "c", "a");
    connect(&mut e, "q", "o1", "c", "b");
    let b = e.bottlenecks(10);
    assert_eq!(b.len(), 2);
    assert!((b[0].shortfall - 50.0).abs() < 1e-9);
    assert!((b[1].shortfall - 10.0).abs() < 1e-9);
    assert_eq!(e.bottlenecks(1).len(), 1);
}

#[test]
fn commands_parse_from_json() {
    let c: Command = serde_json::from_str(r#"{"cmd":"add_object","name":"A","at":[0,0]}"#).unwrap();
    assert!(matches!(c, Command::AddObject { at: Some([0, 0]), .. }));
    let c: Command =
        serde_json::from_str(r#"{"cmd":"set_value","object":"a","entry":"o1","value":5}"#).unwrap();
    assert!(matches!(c, Command::SetValue { value, .. } if value == 5.0));
}

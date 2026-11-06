//! Файлы .pg v2: круговая проверка и отказ от старых форматов.
mod common;
use common::*;
use polygram_core::*;

fn sample() -> Engine {
    let mut e = Engine::new();
    e.set_name("Завод");
    metric_def(&mut e, "tovary", "Товары", MetricKind::Number, "ед.");
    metric_def(&mut e, "vyruchka", "Выручка", MetricKind::Number, "$");
    object_at(&mut e, "p", [0, 0]).unwrap();
    out(&mut e, "p", "o1", "vyruchka", 50.0, Some(500.0));
    object_at(&mut e, "a", [30, 30]).unwrap();
    req(&mut e, "a", "r1", "tovary", 60.0, Some(80.0));
    out(&mut e, "a", "o1", "vyruchka", 3.0, None);
    object_at(&mut e, "b", [30, 60]).unwrap();
    req(&mut e, "b", "r1", "tovary", 40.0, None);
    connect(&mut e, "p", "o1", "a", "r1");
    connect(&mut e, "p", "o1", "b", "r1");
    e
}

#[test]
fn round_trip_preserves_everything() {
    let e = sample();
    let json = e.to_json();
    let back = Engine::from_json(&json).unwrap();
    assert_eq!(back.name(), "Завод");
    assert_eq!(back.stats().objects, 3);
    assert_eq!(back.stats().edges, 2);
    for id in ["p", "a", "b"] {
        let a = serde_json::to_string(&e.object_view(id).unwrap()).unwrap();
        let b = serde_json::to_string(&back.object_view(id).unwrap()).unwrap();
        assert_eq!(a, b, "объект {id}");
    }
    assert_eq!(back.geometry().edges.len(), 2);
    assert_eq!(back.dynamic_state().status, e.dynamic_state().status);
    assert_eq!(back.to_json(), json, "повторная запись даёт тот же файл");
}

#[test]
fn old_or_unknown_formats_are_rejected() {
    let old = r#"{"storage_id":"x","name":"Старый","objects":[],"edges":[]}"#;
    assert_eq!(
        Engine::from_json(old).unwrap_err().code(),
        "unsupported_format"
    );
    let v1 = r#"{"format":"polygram/1","name":"Старый","objects":[],"edges":[]}"#;
    assert_eq!(
        Engine::from_json(v1).unwrap_err().code(),
        "unsupported_format"
    );
    assert_eq!(
        Engine::from_json("не json").unwrap_err().code(),
        "invalid_document"
    );
}

#[test]
fn overlapping_document_is_rejected() {
    let bad = r#"{"format":"polygram/2","name":"x","objects":[
        {"id":"a","name":"a","at":[0,0]},{"id":"b","name":"b","at":[1,0]}]}"#;
    assert_eq!(
        Engine::from_json(bad).unwrap_err().code(),
        "invalid_document"
    );
}

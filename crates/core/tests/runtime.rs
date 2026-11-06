//! Движок в собственном потоке: параллельные клиенты, снимки, остановка.
use polygram_core::*;
use std::thread;

fn add(id: String) -> Command {
    Command::AddObject {
        id: Some(id),
        name: "n".into(),
        kind: NodeKind::Process,
        at: None,
        parent: None,
    }
}

#[test]
fn many_clients_one_writer() {
    let rt = EngineRuntime::spawn(Engine::new());
    let mut joins = Vec::new();
    for t in 0..4 {
        let h = rt.handle();
        joins.push(thread::spawn(move || {
            for k in 0..50 {
                let a = h.apply(add(format!("t{t}n{k}"))).unwrap();
                // «прочитал свою запись»: снимок не старее подтверждённой ревизии
                assert!(h.latest().rev >= a.rev);
            }
        }));
    }
    for j in joins {
        j.join().unwrap();
    }
    let h = rt.handle();
    assert_eq!(h.query(|e| e.stats().objects), Some(200));
    let snap = h.latest();
    assert_eq!(snap.geometry.objects.len(), 200);
    assert_eq!(snap.rev, h.query(|e| e.rev()).unwrap());
    let engine = rt.shutdown().unwrap();
    assert_eq!(engine.stats().objects, 200);
}

#[test]
fn batch_and_errors_cross_the_thread() {
    let rt = EngineRuntime::spawn(Engine::new());
    let h = rt.handle();
    let ok = h
        .apply_batch(vec![add("a".into()), add("b".into())])
        .unwrap();
    assert_eq!(ok.len(), 2);
    let err = h
        .apply_batch(vec![add("c".into()), add("a".into())])
        .unwrap_err();
    assert_eq!(err.0, 1);
    assert_eq!(err.1.code(), "duplicate_id");
    assert_eq!(h.apply(add("a".into())).unwrap_err().code(), "duplicate_id");
}

#[test]
fn stopped_engine_reports_cleanly() {
    let rt = EngineRuntime::spawn(Engine::new());
    let h = rt.handle();
    drop(rt);
    assert_eq!(
        h.apply(add("x".into())).unwrap_err().code(),
        "engine_stopped"
    );
    assert!(h.query(|e| e.rev()).is_none());
}

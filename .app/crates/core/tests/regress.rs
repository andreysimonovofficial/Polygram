//! Регрессионные сценарии Polygram.
//! Прогоняются на чистом ядре без окна egui, без мыши и без ручного ввода агенту.
//! JSONL: одна строка — один шаг. `{"apply": {...}}` применяет команду,
//! `{"expect": {...}}` проверяет состояние. Метрика-значения в формате ядра:
//! `{"kind":"Number","value":50}` / `{"kind":"Null"}`.

mod common;
use common::regress::*;

/// 1. Поток от производителя к двум потребителям: пересчёт статусов и поставок при изменении значения.
const SCENARIO_BASIC_FLOW: &str = r#"
// Создаём производителя и двух потребителей.
{"apply":{"cmd":"add_object","id":"p","name":"Производитель","kind":"process","at":[0,0]}}
{"expect":{"created":"p","at":[0,0],"objects":1}}
{"apply":{"cmd":"add_object","id":"a","name":"Потребитель A","kind":"process","at":[20,0]}}
{"expect":{"created":"a","at":[20,0],"objects":2}}
{"apply":{"cmd":"add_object","id":"b","name":"Потребитель B","kind":"process","at":[40,0]}}
{"expect":{"created":"b","at":[40,0],"objects":3,"no_overlaps":true}}
// Показатель и записи.
{"apply":{"cmd":"add_metric_def","id":"товары","name":"Товары","kind":"number","unit":"ед."}}
{"apply":{"cmd":"add_metric_entry","object":"p","dir":"output","id":"o1","name":"o1","metric_def":"товары","value":{"kind":"Number","value":50},"required":{"kind":"Null"}}}
{"apply":{"cmd":"add_metric_entry","object":"a","dir":"request","id":"r1","name":"r1","metric_def":"товары","value":{"kind":"Null"},"required":{"kind":"Number","value":60}}}
{"apply":{"cmd":"add_metric_entry","object":"b","dir":"request","id":"r1","name":"r1","metric_def":"товары","value":{"kind":"Null"},"required":{"kind":"Number","value":40}}}
// Связи.
{"apply":{"cmd":"connect","from_object":"p","from_entry":"o1","to_object":"a","to_entry":"r1","throughput":100}}
{"apply":{"cmd":"connect","from_object":"p","from_entry":"o1","to_object":"b","to_entry":"r1","throughput":100}}
// Дефицит: 50 < 60+40 → p.o1 и a.r1 в Ожидается, b.r1 получает 40 → Ok.
{"expect":{"statuses":[{"object":"p","entry":"o1","code":"expected"},{"object":"a","entry":"r1","code":"expected"},{"object":"b","entry":"r1","code":"ok"}],"no_overlaps":true}}
// Увеличиваем результат до 100 → спрос покрыт, все Ok.
{"apply":{"cmd":"set_value","object":"p","entry":"o1","value":{"kind":"Number","value":100}}}
{"expect":{"statuses":[{"object":"p","entry":"o1","code":"ok"},{"object":"a","entry":"r1","code":"ok"},{"object":"b","entry":"r1","code":"ok"}],"supplied":[{"object":"a","entry":"r1","value":{"kind":"Number","value":60}},{"object":"b","entry":"r1","value":{"kind":"Number","value":40}}]}}
"#;

/// 2. Перетаскивание объекта (MoveObject) и защита от наложения на занятую клетку.
const SCENARIO_MOVE_OBJECT: &str = r#"
{"apply":{"cmd":"add_object","id":"a","name":"А","kind":"process","at":[0,0]}}
{"expect":{"created":"a","at":[0,0],"objects":1}}
{"apply":{"cmd":"add_object","id":"b","name":"Б","kind":"process","at":[30,0]}}
{"expect":{"created":"b","at":[30,0],"objects":2,"no_overlaps":true}}
{"apply":{"cmd":"move_object","object":"a","at":[5,5]}}
{"expect":{"at":[5,5],"no_overlaps":true}}
{"apply":{"cmd":"move_object","object":"a","at":[30,0]}}
{"expect":{"error":"cell_occupied","objects":2,"no_overlaps":true}}
{"apply":{"cmd":"move_object","object":"a","at":[5,5]}}
{"expect":{"at":[5,5],"no_overlaps":true}}
"#;

/// 3. Дубликат id отвергается.
const SCENARIO_DUPLICATE_ID: &str = r#"
{"apply":{"cmd":"add_object","id":"x","name":"Х","kind":"process","at":[0,0]}}
{"expect":{"created":"x","objects":1}}
{"apply":{"cmd":"add_object","id":"x","name":"Х2","kind":"process","at":[30,0]}}
{"expect":{"error":"duplicate_id","objects":1}}
"#;

/// 4. Цикл в иерархии отвергается (отдел не может стать ребёнком своего ребёнка).
const SCENARIO_CYCLE: &str = r#"
{"apply":{"cmd":"add_object","id":"dept","name":"Отдел","kind":"department","at":[0,0]}}
{"expect":{"created":"dept","objects":1}}
{"apply":{"cmd":"add_object","id":"child","name":"Ребёнок","kind":"process","at":[1,1],"parent":"dept"}}
{"expect":{"created":"child","objects":2,"parents":[{"object":"child","parent":"dept"}]}}
{"apply":{"cmd":"set_object_parent","object":"dept","parent":"child"}}
{"expect":{"error":"object_not_found","parents":[{"object":"child","parent":"dept"}]}}
"#;

/// 5. Узкие места ранжируются по недостаче (спрос − значение результата).
const SCENARIO_BOTTLENECK: &str = r#"
{"apply":{"cmd":"add_metric_def","id":"товары","name":"Товары","kind":"number","unit":"ед."}}
{"apply":{"cmd":"add_object","id":"p","name":"P","kind":"process","at":[0,0]}}
{"apply":{"cmd":"add_object","id":"q","name":"Q","kind":"process","at":[20,0]}}
{"apply":{"cmd":"add_object","id":"c","name":"C","kind":"process","at":[40,0]}}
{"apply":{"cmd":"add_metric_entry","object":"p","dir":"output","id":"o1","name":"o1","metric_def":"товары","value":{"kind":"Number","value":50},"required":{"kind":"Null"}}}
{"apply":{"cmd":"add_metric_entry","object":"q","dir":"output","id":"o1","name":"o1","metric_def":"товары","value":{"kind":"Number","value":90},"required":{"kind":"Null"}}}
{"apply":{"cmd":"add_metric_entry","object":"c","dir":"request","id":"a","name":"a","metric_def":"товары","value":{"kind":"Null"},"required":{"kind":"Number","value":100}}}
{"apply":{"cmd":"add_metric_entry","object":"c","dir":"request","id":"b","name":"b","metric_def":"товары","value":{"kind":"Null"},"required":{"kind":"Number","value":100}}}
{"apply":{"cmd":"connect","from_object":"p","from_entry":"o1","to_object":"c","to_entry":"a","throughput":100}}
{"apply":{"cmd":"connect","from_object":"q","from_entry":"o1","to_object":"c","to_entry":"b","throughput":100}}
{"expect":{"bottlenecks":[{"object":"p","entry":"o1","shortfall":50,"consumers":1},{"object":"q","entry":"o1","shortfall":10,"consumers":1}],"no_overlaps":true}}
"#;

#[test]
fn scenario_basic_flow() {
    assert_scenario(SCENARIO_BASIC_FLOW);
}

#[test]
fn scenario_move_object() {
    assert_scenario(SCENARIO_MOVE_OBJECT);
}

#[test]
fn scenario_duplicate_id() {
    assert_scenario(SCENARIO_DUPLICATE_ID);
}

#[test]
fn scenario_cycle() {
    assert_scenario(SCENARIO_CYCLE);
}

#[test]
fn scenario_bottleneck() {
    assert_scenario(SCENARIO_BOTTLENECK);
}

#[test]
fn all_regression_scenarios() {
    for source in [
        SCENARIO_BASIC_FLOW,
        SCENARIO_MOVE_OBJECT,
        SCENARIO_DUPLICATE_ID,
        SCENARIO_CYCLE,
        SCENARIO_BOTTLENECK,
    ] {
        assert_scenario(source);
    }
}

//! Генератор файла стресс-теста `examples/stress_1000.pg`.
//! 1000 объектов (отделы / процессы / порталы), показатели всех типов и статусов,
//! иерархия отделов и связи между объектами.
//! Запуск: cargo run -p polygram --example gen_stress

use polygram_core::{
    document::{Document, DocEdge, DocObject, DocObjectMetric, DocMetricDef},
    types::{Direction, MetricKind, MetricValue, NodeKind},
};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::fs;

/// Простой xorshift64 PRNG (без внешней зависимости rand).
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        let mut x = seed.wrapping_add(0x9e3779b97f4a7c15);
        if x == 0 {
            x = 1;
        }
        Self(x)
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (self.next_u64() as f64 / u64::MAX as f64) * (hi - lo)
    }
}

fn num(v: f64) -> MetricValue {
    MetricValue::Number(v)
}
fn nul() -> MetricValue {
    MetricValue::Null
}
fn bin(b: bool) -> MetricValue {
    MetricValue::Binary(b)
}
fn pct(v: f64) -> MetricValue {
    MetricValue::Percentage(v)
}
fn mon(v: i64) -> MetricValue {
    MetricValue::Money(v)
}
fn dl(s: &str) -> MetricValue {
    MetricValue::Deadline(s.parse::<DateTime<Utc>>().expect("date"))
}
fn dt(s: &str) -> MetricValue {
    MetricValue::Date(s.parse::<DateTime<Utc>>().expect("date"))
}
fn txt(s: &str) -> MetricValue {
    MetricValue::Text(s.to_owned())
}

fn entry(id: &str, md: &str, dir: Direction, value: MetricValue, required: MetricValue) -> DocObjectMetric {
    DocObjectMetric {
        id: id.to_owned(),
        metric_def: md.to_owned(),
        dir,
        value,
        required,
        max: None,
        fact: nul(),
    }
}

fn main() {
    let mut rng = Rng::new(20261112);
    let mut objects: Vec<DocObject> = Vec::new();
    let mut proc_index: HashMap<String, usize> = HashMap::new();

    let grid_pitch: i32 = 36;
    let grid_cols: i32 = 32;
    let mut grid_idx: i32 = 0;
    fn grid_pos(idx: &mut i32, pitch: i32, cols: i32) -> [i32; 2] {
        let x = (*idx % cols) * pitch;
        let y = (*idx / cols) * pitch;
        *idx += 1;
        [x, y]
    }

    let add = |objects: &mut Vec<DocObject>,
                   id: &str,
                   name: &str,
                   kind: NodeKind,
                   at: [i32; 2],
                   parent: Option<&str>,
                   requests: Vec<DocObjectMetric>,
                   outputs: Vec<DocObjectMetric>| {
        objects.push(DocObject {
            id: id.to_owned(),
            name: name.to_owned(),
            kind,
            at,
            parent: parent.map(|s| s.to_owned()),
            requests,
            outputs,
        });
    };

    // ---------- Иерархический поддерево (зарезервированный регион x >= 2200) ----------
    // dep000/dep001/dep002 — отделы #0..#2; proc000/proc001/proc002 — процессы #0..#2.
    let dmet = |v: f64, r: f64| {
        (
            vec![entry("i1", "dem", Direction::Request, nul(), num(r))],
            vec![entry("o1", "num", Direction::Output, num(v), num(0.0))],
        )
    };
    let pmet = |v: f64, r: f64| {
        (
            vec![entry("i1", "dem", Direction::Request, nul(), num(r))],
            vec![entry("o1", "num", Direction::Output, num(v), num(0.0))],
        )
    };

    let (reqs, outs) = dmet(rng.range(40.0, 160.0), rng.range(40.0, 160.0));
    add(&mut objects, "dep000", "Центральный офис", NodeKind::Department, [2200, 0], None, reqs, outs);
    let (reqs, outs) = dmet(rng.range(40.0, 160.0), rng.range(40.0, 160.0));
    add(&mut objects, "dep001", "Цех А", NodeKind::Department, [2200, 40], Some("dep000"), reqs, outs);
    let (reqs, outs) = dmet(rng.range(40.0, 160.0), rng.range(40.0, 160.0));
    add(&mut objects, "dep002", "Цех Б", NodeKind::Department, [2200, 80], Some("dep000"), reqs, outs);
    let (reqs, outs) = pmet(rng.range(40.0, 160.0), rng.range(40.0, 160.0));
    add(&mut objects, "proc000", "Процесс 000", NodeKind::Process, [2240, 0], Some("dep000"), reqs, outs);
    proc_index.insert("proc000".to_owned(), objects.len() - 1);
    let (reqs, outs) = pmet(rng.range(40.0, 160.0), rng.range(40.0, 160.0));
    add(&mut objects, "proc001", "Процесс 001", NodeKind::Process, [2240, 40], Some("dep001"), reqs, outs);
    proc_index.insert("proc001".to_owned(), objects.len() - 1);
    let (reqs, outs) = pmet(rng.range(40.0, 160.0), rng.range(40.0, 160.0));
    add(&mut objects, "proc002", "Процесс 002", NodeKind::Process, [2240, 80], Some("dep002"), reqs, outs);
    proc_index.insert("proc002".to_owned(), objects.len() - 1);

    // ---------- Остальные отделы (147) на сетке ----------
    for i in 3..150 {
        let id = format!("dep{:03}", i);
        let v = rng.range(40.0, 160.0);
        let r = rng.range(40.0, 160.0);
        let (reqs, outs) = dmet(v, r);
        add(&mut objects, &id, &format!("Отдел {:03}", i), NodeKind::Department, grid_pos(&mut grid_idx, grid_pitch, grid_cols), None, reqs, outs);
    }

    // ---------- Процессы (747) на сетке ----------
    for i in 3..750 {
        let id = format!("proc{:03}", i);
        let mut v = rng.range(40.0, 160.0);
        let mut r = rng.range(40.0, 160.0);
        if i % 50 == 0 {
            v = 100.0;
            r = 100.0;
        }
        let (reqs, outs) = pmet(v, r);
        add(&mut objects, &id, &format!("Процесс {:03}", i), NodeKind::Process, grid_pos(&mut grid_idx, grid_pitch, grid_cols), None, reqs, outs);
        proc_index.insert(id, objects.len() - 1);
    }

    // ---------- Порталы (100) на сетке ----------
    for j in 0..100 {
        let id = format!("port{:03}", j);
        let at = grid_pos(&mut grid_idx, grid_pitch, grid_cols);
        let entry = match j % 10 {
            0 => entry("m1", "bin", Direction::Output, bin(false), nul()),   // Invalid
            1 => entry("m1", "bin", Direction::Output, bin(true), nul()),    // Ok
            2 => entry("m1", "dl", Direction::Output, dl("2026-10-01T00:00:00Z"), nul()), // Deficit (просрочено)
            3 => entry("m1", "dl", Direction::Output, dl("2026-11-12T23:00:00Z"), nul()), // Warning (<1 день)
            4 => entry("m1", "dl", Direction::Output, dl("2026-12-31T23:59:59Z"), nul()), // Ok
            5 => entry("m1", "dt", Direction::Output, nul(), nul()),         // Unknown
            6 => entry("m1", "dt", Direction::Output, dt("2026-11-12T00:00:00Z"), nul()), // Ok
            7 => entry("m1", "txt", Direction::Output, txt("заметка"), nul()), // Unknown (Text)
            8 => entry("m1", "dur", Direction::Output, MetricValue::Duration(3600000), nul()), // Unconnected (Duration)
            _ => entry("m1", "pct", Direction::Output, pct(75.0), nul()),    // Unconnected (Percentage)
        };
        add(&mut objects, &id, &format!("Портал {:03}", j), NodeKind::Portal, at, None, vec![], vec![entry]);
    }

    // ---------- Связи ----------
    let mut edges: Vec<DocEdge> = Vec::new();
    // Кольцевая цепь процессов: proc[i].o1 -> proc[(i+1)%750].i1 (через одно => ~75% связано)
    for i in 0..750 {
        if i % 4 == 0 {
            continue;
        }
        let from = format!("proc{:03}", i);
        let to = format!("proc{:03}", (i + 1) % 750);
        edges.push(DocEdge {
            from_object: from,
            from_entry: "o1".to_owned(),
            to_object: to,
            to_entry: "i1".to_owned(),
            throughput: 100.0,
        });
    }
    // Отделы -> свободные входы процессов (proc[i].i1 без кольцевой связи, i%4==1)
    for k in 0..38 {
        let from = format!("dep{:03}", k);
        let to = format!("proc{:03}", 4 * k + 1);
        edges.push(DocEdge {
            from_object: from,
            from_entry: "o1".to_owned(),
            to_object: to,
            to_entry: "i1".to_owned(),
            throughput: 100.0,
        });
    }
    // Связь внутри отдела: proc000.o1 -> proc001.i1 (кольцевая связь proc000->proc001 пропущена, т.к. 0%4==0)
    edges.push(DocEdge {
        from_object: "proc000".to_owned(),
        from_entry: "o1".to_owned(),
        to_object: "proc001".to_owned(),
        to_entry: "i1".to_owned(),
        throughput: 100.0,
    });

    // ---------- Явные случаи Deficit / Surplus / Ok ----------
    // Выход proc[i].o1 и связанный с ним вход proc[(i+1)%750].i1 (кроме i%4==0).
    for i in 0..750 {
        if i % 4 == 0 {
            continue;
        }
        let val = match i % 60 {
            0 => 10.0,   // Deficit: спрос > значение
            1 => 200.0,  // Surplus: значение > спрос
            2 => 100.0,  // Ok: сбалансировано
            _ => continue,
        };
        let from_out = objects
            .get_mut(proc_index[&format!("proc{:03}", i)])
            .unwrap()
            .outputs
            .iter_mut()
            .find(|e| e.id == "o1")
            .unwrap();
        from_out.value = num(val);
    }
    for i in 0..750 {
        if i % 4 == 0 {
            continue;
        }
        let target = (i + 1) % 750;
        let req = match i % 60 {
            0 => 100.0, // Deficit
            1 => 20.0,  // Surplus
            2 => 100.0, // Ok
            _ => continue,
        };
        let to_req = objects
            .get_mut(proc_index[&format!("proc{:03}", target)])
            .unwrap()
            .requests
            .iter_mut()
            .find(|e| e.id == "i1")
            .unwrap();
        to_req.required = num(req);
    }

    let metric_defs = vec![
        DocMetricDef { id: "num".to_owned(), name: "Выработка".to_owned(), kind: MetricKind::Number, unit: "шт.".to_owned(), api: None },
        DocMetricDef { id: "dem".to_owned(), name: "Потребность".to_owned(), kind: MetricKind::Number, unit: "шт.".to_owned(), api: None },
        DocMetricDef { id: "pct".to_owned(), name: "Загрузка".to_owned(), kind: MetricKind::Percentage, unit: "%".to_owned(), api: None },
        DocMetricDef { id: "bin".to_owned(), name: "Готовность".to_owned(), kind: MetricKind::Binary, unit: "".to_owned(), api: None },
        DocMetricDef { id: "mon".to_owned(), name: "Бюджет".to_owned(), kind: MetricKind::Money, unit: "₽".to_owned(), api: None },
        DocMetricDef { id: "dl".to_owned(), name: "Срок".to_owned(), kind: MetricKind::Deadline, unit: "дд.мм.гггг чч:мм".to_owned(), api: None },
        DocMetricDef { id: "dt".to_owned(), name: "Дата".to_owned(), kind: MetricKind::Date, unit: "дд.мм.гггг".to_owned(), api: None },
        DocMetricDef { id: "txt".to_owned(), name: "Примечание".to_owned(), kind: MetricKind::Text, unit: "".to_owned(), api: None },
        DocMetricDef { id: "dur".to_owned(), name: "Длительность".to_owned(), kind: MetricKind::Duration, unit: "чч:мм".to_owned(), api: None },
    ];

    let doc = Document {
        format: polygram_core::document::FORMAT.to_owned(),
        name: "Сtresstest 1000 объектов".to_owned(),
        metric_defs,
        objects,
        edges,
    };

    let out = "examples/stress_1000.pg";
    fs::write(out, doc.to_json()).expect("write");
    println!("Wrote {}", out);
    println!("Objects: {}", doc.objects.len());
    println!("Edges: {}", doc.edges.len());
    println!("MetricDefs: {}", doc.metric_defs.len());
}

#![allow(dead_code)]
use polygram_core::*;

pub fn object(e: &mut Engine, id: &str) {
    e.apply(Scope::Root, Command::AddObject { id: Some(id.into()), name: id.into(), kind: NodeKind::Process, at: None, parent: None })
        .unwrap();
}

pub fn object_at(e: &mut Engine, id: &str, at: [i32; 2]) -> Result<Applied> {
    e.apply(Scope::Root, Command::AddObject { id: Some(id.into()), name: id.into(), kind: NodeKind::Process, at: Some(at), parent: None })
}

pub fn metric_def(e: &mut Engine, id: &str, name: &str, kind: MetricKind, unit: &str) {
    e.apply(Scope::Root, Command::AddMetricDef { id: Some(id.into()), name: name.into(), kind, unit: unit.into(), api: None }).unwrap();
}

pub fn out(e: &mut Engine, object: &str, entry: &str, metric_def: &str, value: f64, max: Option<f64>) {
    e.apply(
        Scope::Root,
        Command::AddMetricEntry {
            object: object.into(),
            dir: Direction::Output,
            id: Some(entry.into()),
            metric_def: metric_def.into(),
            name: String::new(),
            value,
            required: 0.0,
            max,
            fact: 0.0,
        },
    )
    .unwrap();
}

pub fn req(e: &mut Engine, object: &str, entry: &str, metric_def: &str, required: f64, max: Option<f64>) {
    e.apply(
        Scope::Root,
        Command::AddMetricEntry {
            object: object.into(),
            dir: Direction::Request,
            id: Some(entry.into()),
            metric_def: metric_def.into(),
            name: String::new(),
            value: 0.0,
            required,
            max,
            fact: 0.0,
        },
    )
    .unwrap();
}

pub fn connect(e: &mut Engine, a: &str, am: &str, b: &str, bm: &str) {
    e.apply(
        Scope::Root,
        Command::Connect { from_object: a.into(), from_entry: am.into(), to_object: b.into(), to_entry: bm.into(), throughput: 100.0 },
    )
    .unwrap();
}

pub fn set_value(e: &mut Engine, object: &str, entry: &str, value: f64) {
    e.apply(Scope::Root, Command::SetValue { object: object.into(), entry: entry.into(), value }).unwrap();
}

pub fn set_required(e: &mut Engine, object: &str, entry: &str, required: f64) {
    e.apply(Scope::Root, Command::SetRequired { object: object.into(), entry: entry.into(), required }).unwrap();
}

pub fn status(e: &Engine, object: &str, entry: &str) -> StatusCode {
    let v = e.object_view(object).unwrap();
    v.requests.iter().chain(v.outputs.iter()).find(|m| m.id == entry).unwrap().status
}

pub fn value_of(e: &Engine, object: &str, entry: &str) -> f64 {
    let v = e.object_view(object).unwrap();
    v.requests.iter().chain(v.outputs.iter()).find(|m| m.id == entry).unwrap().supplied
}

/// Простой детерминированный генератор (xorshift64*), чтобы тесты были воспроизводимы.
pub struct Rng(pub u64);
impl Rng {
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    pub fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Два разных объекта не должны пересекаться по клеткам.
pub fn assert_no_overlaps(e: &Engine) {
    let g = e.geometry();
    for (i, a) in g.objects.iter().enumerate() {
        for b in &g.objects[i + 1..] {
            assert!(!a.rect.overlaps(&b.rect), "{} пересекается с {}: {:?} и {:?}", a.id, b.id, a.rect, b.rect);
        }
    }
}

/// Линия состоит только из горизонтальных и вертикальных отрезков.
pub fn assert_orthogonal(points: &[Point]) {
    for w in points.windows(2) {
        assert!(w[0].x == w[1].x || w[0].y == w[1].y, "диагональный отрезок: {:?} -> {:?}", w[0], w[1]);
    }
}

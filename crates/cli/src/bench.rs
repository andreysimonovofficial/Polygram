use polygram_core::*;
use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn ms(d: Duration) -> String {
    format!("{:.1} мс", d.as_secs_f64() * 1000.0)
}

fn us(d: Duration) -> String {
    format!("{:.1} мкс", d.as_secs_f64() * 1e6)
}

fn percentile(sorted: &[Duration], p: f64) -> Duration {
    if sorted.is_empty() {
        return Duration::ZERO;
    }
    sorted[((sorted.len() - 1) as f64 * p) as usize]
}

fn metric(object: &str, dir: Direction, id: &str, metric_def: &str, value: f64, required: f64, max: Option<f64>) -> Command {
    Command::AddMetricEntry {
        object: object.to_owned(),
        dir,
        id: Some(id.to_owned()),
        metric_def: metric_def.to_owned(),
        name: id.to_owned(),
        value,
        required,
        max,
        fact: 0.0,
    }
}

fn build_commands(n: usize, rng: &mut Rng) -> Vec<Command> {
    let mut cmds = Vec::with_capacity(n * 6 + n * 2);
    cmds.push(Command::AddMetricDef { id: Some("tovary".into()), name: "Товары".into(), kind: Default::default(), unit: "ед.".into(), api: None });
    for i in 0..n {
        let id = format!("n{i}");
        cmds.push(Command::AddObject { id: Some(id.clone()), name: id.clone(), kind: NodeKind::Process, at: None, parent: None });
        for k in 1..=2 {
            cmds.push(metric(&id, Direction::Request, &format!("r{k}"), "tovary", 0.0, 5.0 + rng.below(45) as f64, None));
            cmds.push(metric(&id, Direction::Output, &format!("o{k}"), "tovary", rng.below(100) as f64, 0.0, Some(150.0)));
        }
    }
    let mut seen: HashSet<(usize, usize, usize, usize)> = HashSet::new();
    let mut edges = 0;
    while edges < n * 2 && n > 1 {
        let a = rng.below(n);
        let b = (a + 1 + rng.below(40)).min(n - 1);
        if a == b {
            continue;
        }
        let (ma, mb) = (1 + rng.below(2), 1 + rng.below(2));
        if !seen.insert((a, ma, b, mb)) {
            continue;
        }
        cmds.push(Command::Connect {
            from_object: format!("n{a}"),
            from_entry: format!("o{ma}"),
            to_object: format!("n{b}"),
            to_entry: format!("r{mb}"),
            throughput: 100.0,
        });
        edges += 1;
    }
    cmds
}

pub fn bench(n: usize) -> i32 {
    println!("Замер ядра Polygram: {n} объектов, по 2 входа и 2 выхода, ~{} связей\n", n * 2);
    let mut rng = Rng(0x00C0FFEE);
    let cmds = build_commands(n, &mut rng);
    let total_cmds = cmds.len();

    let mut e = Engine::with_capacity(n, n * 2);
    let t = Instant::now();
    e.apply_batch(Scope::Root, cmds).expect("построение графа");
    let build = t.elapsed();
    println!("  Построение графа ({total_cmds} команд, расчёт и линии)  {}", ms(build));

    let t = Instant::now();
    e.apply(Scope::Root, Command::AutoLayout).unwrap();
    println!("  Автораскладка + маршруты всех связей                  {}", ms(t.elapsed()));

    let t = Instant::now();
    e.apply(Scope::Root, Command::RerouteAll).unwrap();
    println!("  Пересчёт маршрутов всех связей                        {}", ms(t.elapsed()));

    // Инкрементальные обновления значений (как приходят от агентов и коннекторов).
    let updates = 10_000;
    let mut samples = Vec::with_capacity(updates);
    for k in 0..updates {
        let cmd = Command::SetValue {
            object: format!("n{}", rng.below(n)),
            entry: format!("o{}", 1 + rng.below(2)),
            value: ((k * 7) % 160) as f64,
        };
        let t = Instant::now();
        e.apply(Scope::Root, cmd).unwrap();
        samples.push(t.elapsed());
    }
    samples.sort();
    let avg = samples.iter().sum::<Duration>() / samples.len() as u32;
    println!(
        "  Изменение значения ×{updates}: среднее {}, p99 {}, худшее {}",
        us(avg),
        us(percentile(&samples, 0.99)),
        us(*samples.last().unwrap())
    );

    // Перетаскивание объектов по сетке.
    let mut moves = Vec::new();
    for _ in 0..2000 {
        let id = format!("n{}", rng.below(n));
        let at = e.object_view(&id).unwrap().at;
        let t = Instant::now();
        if e.apply(Scope::Root, Command::MoveObject { object: id, at: [at[0] + 1, at[1]] }).is_ok() {
            moves.push(t.elapsed());
        }
    }
    moves.sort();
    if !moves.is_empty() {
        let avg = moves.iter().sum::<Duration>() / moves.len() as u32;
        println!(
            "  Перемещение объекта ×{}: среднее {}, p99 {}",
            moves.len(),
            us(avg),
            us(percentile(&moves, 0.99))
        );
    }

    let t = Instant::now();
    let dynamic = e.dynamic_state();
    let t_dyn = t.elapsed();
    let t = Instant::now();
    let geo = e.geometry();
    let t_geo = t.elapsed();
    println!("  Снимок чисел для окна ({} показателей)            {}", dynamic.value.len(), ms(t_dyn));
    println!("  Снимок геометрии для окна ({} объектов, {} линий) {}", geo.objects.len(), geo.edges.len(), ms(t_geo));

    let t = Instant::now();
    let bn = e.bottlenecks(20);
    println!("  Поиск узких мест (найдено {})                        {}", bn.len(), us(t.elapsed()));

    let t = Instant::now();
    let fresh = e.recomputed_state();
    let t_full = t.elapsed();
    let ok = fresh == e.dynamic_state();
    println!("  Полный пересчёт с нуля (для сверки)                   {}", ms(t_full));
    println!("  Сверка инкрементального расчёта с полным: {}", if ok { "СОВПАДАЕТ" } else { "РАСХОЖДЕНИЕ!" });

    // Через рантайм: движок в своём потоке, клиент ждёт ответ и публикуется снимок.
    let rt = EngineRuntime::spawn(e);
    let h = rt.handle();
    let mut via = Vec::new();
    for k in 0..1000 {
        let cmd = Command::SetValue {
            object: format!("n{}", rng.below(n)),
            entry: format!("o{}", 1 + rng.below(2)),
            value: ((k * 11) % 170) as f64,
        };
        let t = Instant::now();
        h.apply(cmd).unwrap();
        via.push(t.elapsed());
    }
    via.sort();
    println!(
        "  Через поток движка ×1000 (с публикацией снимка): среднее {}, p99 {}",
        us(via.iter().sum::<Duration>() / via.len() as u32),
        us(percentile(&via, 0.99))
    );
    drop(rt);

    println!("\nЕсли что-то из этого кажется медленным — пришли этот вывод целиком.");
    i32::from(!ok)
}

use polygram_core::*;
use std::collections::BTreeMap;
use std::fs;

pub fn summary(e: &Engine) {
    let s = e.stats();
    println!("Проект «{}»: объектов {}, показателей {}, связей {}, ревизия {}", e.name(), s.objects, s.object_metrics, s.edges, s.rev);
    let mut by_status: BTreeMap<&str, usize> = BTreeMap::new();
    for m in e.hierarchy().live_object_metrics() {
        *by_status.entry(e.values().status[m.idx()].label_ru()).or_default() += 1;
    }
    for (k, v) in by_status {
        println!("  {k}: {v}");
    }
    let bn = e.bottlenecks(5);
    if bn.is_empty() {
        println!("Узких мест нет.");
    } else {
        println!("Узкие места (по убыванию недостачи):");
        for b in bn {
            let h = e.hierarchy();
            let obj = h.object(ObjectIx(b.object));
            let entry = h.object_metric(EntryIx(b.entry));
            let md = h.metric_def(entry.metric_def);
            println!("  {} / {}: не хватает {:.1} {} (потребителей: {})", obj.name, md.name, b.shortfall, md.unit, b.consumers);
        }
    }
}

pub fn run(args: &[String]) -> i32 {
    let mut script: Option<&String> = None;
    let (mut load, mut save): (Option<&String>, Option<&String>) = (None, None);
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--load" => {
                load = args.get(i + 1);
                i += 1;
            }
            "--save" => {
                save = args.get(i + 1);
                i += 1;
            }
            _ => script = Some(&args[i]),
        }
        i += 1;
    }
    let Some(script) = script else {
        crate::usage();
        return 2;
    };
    let mut engine = match load {
        Some(p) => match crate::load_engine(p) {
            Ok(e) => e,
            Err(m) => {
                eprintln!("Ошибка: {m}");
                return 1;
            }
        },
        None => Engine::new(),
    };
    let text = match fs::read_to_string(script) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Ошибка: не удалось прочитать «{script}»: {e}");
            return 1;
        }
    };
    let mut failed = 0;
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let cmd: Command = match serde_json::from_str(line) {
            Ok(c) => c,
            Err(e) => {
                println!("строка {}: не понял команду ({e})", n + 1);
                failed += 1;
                continue;
            }
        };
        match engine.apply(Scope::Root, cmd) {
            Ok(a) => match a.created {
                Some(id) => println!("строка {}: готово, создано «{id}» (ревизия {})", n + 1, a.rev),
                None => println!("строка {}: готово (ревизия {})", n + 1, a.rev),
            },
            Err(e) => {
                println!("строка {}: ОТКАЗ [{}] {e}", n + 1, e.code());
                failed += 1;
            }
        }
    }
    println!();
    summary(&engine);
    if let Some(path) = save {
        match fs::write(path, engine.to_json()) {
            Ok(()) => println!("\nСохранено в «{path}»"),
            Err(e) => {
                eprintln!("Ошибка записи «{path}»: {e}");
                return 1;
            }
        }
    }
    i32::from(failed > 0)
}

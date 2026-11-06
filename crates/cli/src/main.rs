use polygram_core::*;
use std::{env, fs};

mod bench;
mod run_cmd;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("bench") => bench::bench(args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1000)),
        Some("run") => run_cmd::run(&args[1..]),
        Some("open") => open(args.get(1)),
        _ => {
            usage();
            0
        }
    };
    std::process::exit(code);
}

fn usage() {
    println!("Polygram — консольный терминал ядра\n");
    println!("  pg bench [узлов]                       замер скорости (по умолчанию 1000)");
    println!("  pg run файл.jsonl [--load f.pg] [--save f.pg]   выполнить команды из файла");
    println!("  pg open файл.pg                        сводка по проекту и узкие места");
}

fn load_engine(path: &str) -> std::result::Result<Engine, String> {
    let text =
        fs::read_to_string(path).map_err(|e| format!("не удалось прочитать «{path}»: {e}"))?;
    Engine::from_json(&text).map_err(|e| e.to_string())
}

fn open(path: Option<&String>) -> i32 {
    let Some(path) = path else {
        usage();
        return 2;
    };
    match load_engine(path) {
        Ok(e) => {
            run_cmd::summary(&e);
            0
        }
        Err(msg) => {
            eprintln!("Ошибка: {msg}");
            1
        }
    }
}

# Polygram v2

Граф процессов и показателей, которым одинаково управляют человек и ИИ-агент.
Чистовик на Rust: ядро отдельно от окна, всё через команды.

```
cargo test -p polygram-core                       # 34 проверки ядра
cargo run --release -p polygram-cli -- bench 10000 # замер скорости
cargo run --release -p polygram-cli -- run examples/demo.jsonl
```

Начни с `docs/VISION.md` и `docs/ARCHITECTURE.md`.

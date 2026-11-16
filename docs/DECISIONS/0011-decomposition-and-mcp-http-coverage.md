# 0011. Декомпозиция мега-компонентов и покрытие GUI действиями через MCP/HTTP

**Дата:** 2026-11-16 · **Статус:** принято

## Проблема

Несколько файлов превышают порог «один файл — одна ответственность» (>500 строк), а часть действий,
доступных из GUI, не имеет dédiéрованных точек входа для агентов (MCP/HTTP).

## Изменения

1. `crates/polygram/src/main.rs` (560 строк) разложен:
   - регистрация шрифтов → `crates/polygram/src/app/fonts.rs`;
   - сохранение скриншота → `crates/polygram/src/app/snapshot.rs`;
   - `main.rs` оставит только точку входа `main()`, иконку, окно egui и диспетчер `ui()`.
2. `crates/polygram/src/interaction/canvas.rs` (550 строк): тесты вынесены в
   `crates/polygram/src/interaction/canvas_tests.rs`; сам файл стал 422 строки.
3. `crates/api/src/mcp.rs` (574 строк) разделён на модули в `crates/api/src/mcp/`:
   - `mod.rs` — ядро JSON-RPC 2.0 (`McpServer`, `run`, `handle_message`);
   - `tools.rs` — перечень и диспетчер инструментов (`handle_tools_list`, `handle_tools_call`, `apply_command`);
   - `proxy.rs` — HTTP-прокси в `polygram-api.exe`;
   - `result.rs` — формирование JSON-RPC ответов инструментов.
4. Покрытие GUI действиями через MCP/HTTP:
   - новые MCP-инструменты: `chat_post` (сообщение в ленту чата), `document_open` (открыть .pg),
     `document_save_as` (сохранить граф);
   - все мутации графа (объекты, метрики, связи, раскладка, камера, скриншот-режим) и так доступны
     через `Command` — HTTP `POST /api/commands` / `/api/commands/batch` и MCP `execute_command`;
   - HTTP уже предоставляет `/api/document/open`, `/api/document/save-as`, `/api/chat/messages`.

## Граница ответственности (ARCHITECTURE.md правило #1)

Ядро (`core`) и API-сервер — headless: они не знают об окне egui. GUI-специфичное состояние
(переключение панелей, тема, снимок самого окна `polygram.exe`) живёт только в оболочке GUI и не
выставляется через headless API, т.к. API-сервер — отдельный процесс. Агенту через MCP/HTTP доступны
все actions над графом и документами; скриншот окна и настройка оформления остаются локальными
действиями пользователя в GUI.

## Проверка

- `cargo test -p polygram-core --test walls` — 2/2 (стены: ≤1000 строк, core headless).
- `cargo test --workspace` — зелёно, кроме предсуществующего `canvas::routing::layer_tests`
  (падает и на чистом master, не関連 к данному изменению).
- Файлы исходников (без тестов/примеров):最大 579 строк — `canvas/routing/tests.rs` (тест),
  остальные <500.

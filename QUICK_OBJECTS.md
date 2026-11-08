# Шпаргалка: быстрое создание объектов (HTTP / MCP) в GUI Polygram

## HTTP API (порт 8080, внутри polygram.exe)

### 1. POST /api/objects — быстрый способ
```bash
curl -X POST http://127.0.0.1:8080/api/objects \
  -H "Content-Type: application/json" \
  -d '{"name":"Сборочный цех","kind":"department"}'
```
Поля:
- `name` (обязательно) — имя объекта
- `kind` — `process` | `department` | `portal` (по умолчанию `process`)
- `id` — внешний id (опционально, генерируется自动)
- `at` — `[x, y]` клетка (опционально, подбирается автоматически)
- `parent` — внешний id родительского отдела (None = корень)

Ответ: `{ ok: true, rev, created: "<id>", at: [x,y] }`

### 2. POST /api/commands — универсальный (любая команда ядра)
```bash
curl -X POST http://127.0.0.1:8080/api/commands \
  -H "Content-Type: application/json" \
  -d '{"cmd":"add_object","name":"Сборочный цех","kind":"department"}'
```
Это то же самое, что и `/api/objects`, но через общую точку входа (`cmd` = имя команды в snake_case).

### 3. GET /api/objects — список всех объектов
```bash
curl http://127.0.0.1:8080/api/objects
```

### 4. GET /api/objects/:id — один объект
```bash
curl http://127.0.0.1:8080/api/objects/429
```

## MCP сервер (stdio)

Запускается вместе с GUI. Команды через JSON-RPC 2.0:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "apply_command",
    "arguments": { "cmd": "add_object", "name": "Сборочный цех", "kind": "department" }
  }
}
```

## Как объект появляется в GUI

GUI и API делят одно состояние через `ApiService` → один `EngineHandle`. Любая команда через HTTP или MCP вызывает `engine.apply(cmd)`, после чего ревизия растёт и снимок публикуется. GUI читает `self.runtime.latest()` каждый кадр — объект на холсте появляется сразу.

## Проверка, что сервер запущен

```bash
curl http://127.0.0.1:8080/api/snapshot
```
Должен вернуть JSON с `rev` и объектами.

## Типичные проблемы

1. **Порт 8080 занят / сервер молчит** — старый процесс polygram/polygram-api не завершён:
   ```powershell
   Get-Process polygram, polygram-api | Stop-Process -Force
   ```
   Затем перезапустить: `cargo run -p polygram`

2. **Объект не виден на холсте** — убедиться, что GUI перезапущен после создания (загружает `polygram_auto.pg` из папки рядом с exe) или что API и GUI это один процесс.

3. **Не могу подключиться к 8080 извне** — сервис биндится на `127.0.0.1` (только localhost).

## Примеры команд ядра (cmd)

- `add_object` — создать объект
- `remove_object` — удалить
- `rename_object` — переименовать
- `move_object` — переместить на клетку `[x,y]`
- `add_metric_def` — создать глобальный показатель
- `add_metric_entry` — добавить запись показателя в объект
- `set_value` / `set_required` / `set_max` / `set_fact` — значения
- `connect` / `disconnect` — связь между объектами

Все команды описаны в `crates/core/src/command.rs` (enum `Command`, тег `cmd`, `rename_all = "snake_case"`).
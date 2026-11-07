# Polygram MCP Server

Lокальный MCP-сервер (JSON-RPC 2.0 над stdio) для подключения из Claude Desktop, Open WebUI и других MCP-клиентов.

## Сборка

```powershell
cargo build -p polygram-mcp
```

Бинарник появится в `target\debug\polygram-mcp.exe` (или `target\release\`).

## Подключение

### Claude Desktop

Файл конфигурации: `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "polygram": {
      "command": "C:\\Dev\\Polygram\\target\\debug\\polygram-mcp.exe",
      "args": []
    }
  }
}
```

После перезапуска Claude Desktop в списке инструментов появятся:
- `execute_command` — выполнить команду Polygram (JSON, поле `cmd`)
- `get_snapshot` — последний снимок состояния
- `get_objects` — список всех объектов
- `get_stats` — статистика графа

### Open WebUI

Админ-панель → Settings → MCP → добавить stdio-сервер с командой `polygram-mcp.exe`.

## Пример команд

```json
{"cmd":"add_object","id":"A","name":"Процесс А","kind":"process"}
{"cmd":"add_metric_def","id":"Деньги","name":"Деньги","kind":"number","unit":"$"}
{"cmd":"add_metric_entry","object":"A","dir":"output","metric_def":"Деньги","name":"Деньги","value":100.0}
{"cmd":"connect","from_object":"A","from_entry":"Деньги","to_object":"B","to_entry":"Деньги","throughput":100.0}
```

## Работа Polygram API (альтернатива, HTTP)

Также доступен HTTP API на порту 8080 (`cargo run -p polygram-api`):
- `GET /api/snapshot`
- `GET /api/stats`
- `GET /api/objects`
- `POST /api/commands`

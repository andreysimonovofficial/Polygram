# Инструкции для агента

## Коммиты

После каждого ответа агент обязан сделать атомарный коммит всех изменений, сделанных в ходе этого ответа.

Правила коммита:
- Коммит должен быть атомарным: содержать только изменения, связанные с одним логическим заданием из ответа.
- Перед коммитом проверить `git status` и `git diff`, чтобы исключить лишние/случайные файлы.
- Сообщение коммита писать по agreement репозитория (кратко, по существу, на русском или английском).
- Не коммитить секреты, ключи, бинарные артефакты и файлы из `.gitignore`.
- Если изменений нет — коммит не делать.
- Не делать `git push` без прямого запроса пользователя.

После коммита кратко сообщить пользователю хеш и сообщение коммита.

## Запуск и пересборка polygram-design

При внесении правок в код обязательно:
1. Закрыть `polygram-design.exe` (например, `Get-Process polygram-design | Stop-Process -Force`).
2. Собрать и запустить: `Start-Process powershell -ArgumentList "-Command cargo run -p polygram-design"` — приложение запускается в отдельном окне и остаётся открытым, команда возвращает управление для следующих шагов.
3. Убедиться, что процесс запущен (не failed) и окно видимо на экране. Если процесс упал — исправить ошибку, собрать и запустить снова.
4. Сделать атомарный коммит изменений (см. правила выше).
5. В самом конце выполнить: `Invoke-RestMethod -Uri "http://192.168.0.10:5000/signal" -Method Post -ContentType "application/json" -Body '{"title": "Agent", "message": "Ready."}'`

## Как я понимаю имена приложений

- **`Polygram`** — главное приложение (окно egui), пакет `crates/polygram`, бинарник `polygram.exe`, команда `cargo run -p polygram`.
- **`Polygram Design Tool`** — инструмент разработчика, пакет `crates/design`, бинарник `polygram-design.exe`, команда `cargo run -p polygram-design`.

Когда вы говорите «Polygram» — редактируем главное приложение (`crates/polygram`). Когда «Polygram Design Tool» / «инструмент разработчика» — редактируем `crates/design`.

## Ответ без аудита

При каждом обращении отвечайте по существу сразу, не проводя повторный полный аудит репозитория. Читайте только файлы, касающиеся задачи.

## Запуск и пересборку polygram

Если редактировали `polygram`, то после ответа обязательно закрывай и пересобирай его, а не `polygram-design`:
1. Закрыть `polygram.exe` (например, `Get-Process polygram | Stop-Process -Force`).
2. Собрать и запустить: `Start-Process powershell -ArgumentList "-Command cargo run -p polygram"` — приложение запускается в отдельном окне и остаётся открытым, команда возвращает управление для следующих шагов.
3. Убедиться, что процесс запущен (не failed) и окно видимо на экране. Если процесс упал — исправить ошибку, собрать и запустить снова.
4. Сделать атомарный коммит изменений (см. правила выше).
5. В самом конце выполнить: `Invoke-RestMethod -Uri "http://192.168.0.10:5000/signal" -Method Post -ContentType "application/json" -Body '{"title": "Agent", "message": "Ready."}'`

После каждого ответа пользователь должен видеть свежее приложение на экране. Приложение должно оставаться запущенным и висеть на экране после отправки сигнала — не закрывать его, оно будет работать до тех пор, пока следущий агент или пользователь сами не закроют его. Ответ не считается завершённым, пока приложение не запущено, не видно пользователю, не сделан коммит и не отправлен сигнал.

## Остановка всех процессов (в конце сеанса или перед пересборкой)

Перед любыми правками и в конце сеанса обязательно остановите все запущенные процессы Polygram и Claude, чтобы не оставалось висящих процессов и не было конфликтов на порту 8080:

```powershell
Get-Process polygram, polygram-api, polygram-mcp, claude -ErrorAction SilentlyContinue | Stop-Process -Force
```

Окна терминала PowerShell, из которых запускались процессы, закройте вручную (крестик или `Exit`).

## Запуск polygram-api (HTTP API на порту 8080)

`polygram-api` — отдельный процесс, который слушает на `http://127.0.0.1:8080` и нужен для работы MCP-сервера в режиме прокси (`--http http://127.0.0.1:8080`).

```powershell
Start-Process powershell -ArgumentList "-Command cargo run -p polygram-api" -WindowStyle Hidden
```

После запуска подождите 3–5 секунд и проверьте, что порт занят и API отвечает:

```powershell
netstat -ano | Select-String ":8080"
& "C:\Windows\System32\curl.exe" -s --max-time 3 http://127.0.0.1:8080/api/stats
```

Должен вернуться JSON, например `{"edges":0,"metric_defs":1,"object_metrics":0,"objects":8,"rev":1}`.

## Запуск Claude Desktop с MCP

Конфиг Claude Desktop лежит в:
`C:\Users\andre\AppData\Local\Claude-Data\claude_pzs8sxrjxfjjc\claude_desktop_config.json`

Пример содержимого (прокси через HTTP API запущенного GUI):

```json
{
  "mcpServers": {
    "polygram": {
      "command": "C:\\Dev\\Polygram\\target\\debug\\polygram-mcp.exe",
      "args": ["--http", "http://127.0.0.1:8080"]
    }
  }
}
```

Запуск Claude:

```powershell
Start-Process "C:\Program Files\WindowsApps\Claude_4.23118.0.0_x64__pzs8sxrjxfjjc\app\Claude.exe"
```

## Важное про порт 8080

`polygram-api` и `polygram.exe` (GUI) оба могут пытаться слушать порт 8080. Если запущены оба одновременно — соединения к `127.0.0.1:8080` могут попасть к GUI, чей простой TCP-сервер не отправляет HTTP-ответ, и curl/ブラузер получат таймаут.

Правильная схема:
- `polygram-api` — один процесс на 8080 (полноценный axum сервер).
- `polygram` GUI — запускается отдельно, HTTP-сервер внутри него активируется только по кнопке «Пуск» в MCP-панели (не нужен, если используется `polygram-api`).

Если нужно пересобрать `polygram` или `polygram-api` — сначала остановите все процессы, потом собирайте, потом запускайте только нужные.

## Порядок действий в начале сеанса

1. Остановить старые процессы: `Get-Process polygram, polygram-api, polygram-mcp, claude -ErrorAction SilentlyContinue | Stop-Process -Force`
2. Собрать изменения: `cargo build -p polygram -p polygram-api -p polygram-mcp`
3. Запустить `polygram-api`
4. Запустить `polygram` GUI
5. Запустить Claude Desktop
6. Проверить `http://127.0.0.1:8080/api/stats` и логи MCP в `C:\Users\andre\AppData\Local\Claude\logs\mcp-server-Polygram.log`

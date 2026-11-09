# План: вынести Polygram Design Tool в отдельный репозиторий

**Цель:** `C:\Dev\Polygram` — только приложение Polygram (polygram + core + canvas + api + mcp + cli). `C:\Dev\Polygram-Deisgn-Tool` — только инструмент дизайна (design + core + canvas). Оба собираются и запускаются независимо, у каждого свой GitHub-репозиторий.

## Решения

1. Папка инструмента: `C:\Dev\Polygram-Deisgn-Tool` (написание как у пользователя, с опечаткой "Deisgn").
2. В новый репозиторий переносится `crates/design` вместе с копиями `crates/core` и `crates/canvas` — у инструмента внешние зависимости только от этих двух крейт (см. `crates/design/Cargo.toml:21-22`), поэтому копия делает его самодостаточным. Код не дублируется логически: каждая крейта живёт в одном репозитории.
3. В старом репозитории `crates/design` удаляется из `members` в `Cargo.toml` и директория `crates/design` удаляется. Остальное без изменений.
4. Новый репозиторий — новая git-история (`git init`). Полное вычищение истории дизайна из старого репозитория (filter-repo) не делается — это отдельная операция по запросу.
5. GitHub-репозиторий для инструмента создаёт пользователь вручную (у агента нет прав на создание репозиториев на GitHub); затем добавляет remote и пушит.

## Шаги выполнения

1. Остановить процессы:
   ```powershell
   Get-Process polygram, polygram-design, polygram-api, polygram-mcp, claude -ErrorAction SilentlyContinue | Stop-Process -Force
   ```
2. Создать папку и структуру нового репозитория:
   ```powershell
   mkdir C:\Dev\Polygram-Deisgn-Tool
   mkdir C:\Dev\Polygram-Deisgn-Tool\crates
   mv C:\Dev\Polygram\crates\design C:\Dev\Polygram-Deisgn-Tool\crates\design
   cp -r C:\Dev\Polygram\crates\core C:\Dev\Polygram-Deisgn-Tool\crates\core
   cp -r C:\Dev\Polygram\crates\canvas C:\Dev\Polygram-Deisgn-Tool\crates\canvas
   ```
3. Создать `C:\Dev\Polygram-Deisgn-Tool\Cargo.toml`:
   ```toml
   [workspace]
   resolver = "2"
   members = ["crates/core", "crates/canvas", "crates/design"]

   [workspace.package]
   version = "0.1.0"
   edition = "2021"
   license = "UNLICENSED"

   [profile.release]
   opt-level = 3
   lto = "thin"
   codegen-units = 1

   [profile.dev]
   opt-level = 1
   ```
   `crates/design/Cargo.toml` не менять (уже ссылается на `../core`, `../canvas`).
4. В `C:\Dev\Polygram\Cargo.toml` убрать `"crates/design"` из `members`; удалить `C:\Dev\Polygram\crates\design`.
5. Собрать оба репозитория:
   ```powershell
   cargo build -p polygram                      # в C:\Dev\Polygram
   cargo build -p polygram-design               # в C:\Dev\Polygram-Deisgn-Tool
   ```
6. Запустить и проверить окна (по AGENTS.md):
   ```powershell
   Start-Process powershell -ArgumentList "-Command cargo run -p polygram" -WorkDirectory C:\Dev\Polygram
   Start-Process powershell -ArgumentList "-Command cargo run -p polygram-design" -WorkDirectory C:\Dev\Polygram-Deisgn-Tool
   ```
   Убедиться, что оба окна открылись и не упали.
7. Коммиты (атомарно, без push):
   - старый репо: `git add -A; git commit -m "refactor: вынести polygram-design в отдельный репозиторий"`
   - новый репо: `git init; git add -A; git commit -m "chore: начальная версия polygram-design"`
8. Пользователь создаёт GitHub-репозиторий для инструмента, добавляет remote и пушит:
   ```powershell
   git remote add origin <URL>
   git branch -M main
   git push -u origin main
   ```
9. В конце: `Invoke-RestMethod -Uri "http://192.168.0.10:5000/signal" -Method Post -ContentType "application/json" -Body '{"title": "Agent", "message": "Ready."}'`

## Проверка

- `cargo build -p polygram` проходит без ошибок (design больше не в workspace).
- `cargo build -p polygram-design` проходит в новой папке.
- Оба приложения запускаются и отображают окна.
- У каждого репозитория свой `.git`, нет общих зависимостей по путям вне репозитория.

## Открытые вопросы (решить пользователю)

- Имя GitHub-репозитория для инструмента и его URL.
- Нужно ли вычищать историю `crates/design` из старого репозитория (`git filter-repo`).

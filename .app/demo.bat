@echo off
chcp 65001 >nul
cd /d "%~dp0"
echo === Mini-factory: baseline ===
cargo run --release -p polygram-cli -- run examples/demo.jsonl --save demo.pg
echo.
echo === After adding capacity ===
cargo run --release -p polygram-cli -- run examples/demo_fix.jsonl --load demo.pg
pause

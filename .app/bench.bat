@echo off
chcp 65001 >nul
cd /d "%~dp0"
cargo run --release -p polygram-cli -- bench 1000
cargo run --release -p polygram-cli -- bench 10000
pause

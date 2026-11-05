@echo off
chcp 65001 >nul
cd /d "%~dp0"
cargo test -p polygram-core
pause

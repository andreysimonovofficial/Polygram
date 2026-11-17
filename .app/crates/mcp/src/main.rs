//! MCP сервер Polygram (JSON-RPC 2.0 над stdio).
//! Два режима:
//!   • без флагов — standalone, своё ядро (для запуска отдельно от GUI);
//!   • --http <url> — прокси в HTTP API запущенного polygram.exe (единое ядро).

use polygram_api::{ApiService, mcp::McpServer};
use std::sync::{Arc, atomic::AtomicBool};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // --http <url> — прокси в запущенный polygram.exe (единое ядро).
    let http_base = if args.len() >= 3 && args[1] == "--http" {
        Some(args[2].clone())
    } else {
        None
    };

    let running = Arc::new(AtomicBool::new(true));

    if let Some(base) = &http_base {
        // Прокси-режим: нет своего ядра, все команды идут в polygram.exe.
        let api = ApiService::default(); // используется только для совместимости интерфейса
        McpServer::new(api).with_http(base.clone()).run(running);
    } else {
        // Standalone — своё ядро.
        let api = ApiService::default();
        McpServer::new(api).run(running);
    }
}

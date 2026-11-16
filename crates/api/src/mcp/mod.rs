//! MCP сервер Polygram (JSON-RPC 2.0 over stdio).
//! Разбито на модули:
//! - `proxy` — HTTP-прокси в polygram-api.exe.
//! - `result` — формирование JSON-RPC ответов инструментов.
//! - `tools` — перечень и диспетчер инструментов MCP.

use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::sync::mpsc;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub mod proxy;
pub mod result;
pub mod tools;

use crate::mcp::tools::{handle_initialize, handle_tools_call, handle_tools_list};

use super::ApiService;

/// Событие о выполненной MCP-команде — пересылается в GUI для лога.
#[derive(Clone, Debug)]
pub struct McpEvent {
    pub cmd: String,
    pub result: String,
}

pub struct McpServer {
    api: ApiService,
    mcp_tx: Option<mpsc::Sender<McpEvent>>,
    http_base: Option<String>,
}

impl McpServer {
    pub fn new(api: ApiService) -> Self {
        Self {
            api,
            mcp_tx: None,
            http_base: None,
        }
    }

    pub fn with_events(mut self, tx: mpsc::Sender<McpEvent>) -> Self {
        self.mcp_tx = Some(tx);
        self
    }

    /// Прокси-режим: команды пересылаются в HTTP API запущенного polygram.exe.
    pub fn with_http(mut self, base_url: String) -> Self {
        self.http_base = Some(base_url);
        self
    }

    pub fn run(self, running: Arc<AtomicBool>) {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut out = stdout.lock();

        for line in stdin.lock().lines() {
            if !running.load(Ordering::SeqCst) {
                break;
            }
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };
            if line.trim().is_empty() {
                continue;
            }

            let response = self.handle_message(&line);
            if let Some(resp) = response {
                let resp_str = serde_json::to_string(&resp).unwrap();
                let _ = writeln!(out, "{}", resp_str);
                let _ = out.flush();
            }
        }
    }

    fn handle_message(&self, line: &str) -> Option<Value> {
        let req: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => return Some(jsonrpc_error(None, -32700, "Parse error")),
        };

        let method = req.get("method")?.as_str()?;
        let id = req.get("id").cloned();

        // Уведомления (без id) не ждут ответа.
        if id.is_none() {
            return None;
        }

        let params = req.get("params").cloned().unwrap_or(json!({}));

        let result = match method {
            "initialize" => handle_initialize(params),
            "tools/list" => handle_tools_list(),
            "tools/call" => handle_tools_call(self, params),
            _ => return Some(jsonrpc_error(id.as_ref(), -32601, "Method not found")),
        };

        Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result,
        }))
    }
}

fn jsonrpc_error(id: Option<&Value>, code: i32, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id.cloned(),
        "error": { "code": code, "message": message }
    })
}

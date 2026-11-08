//! MCP сервер Polygram (JSON-RPC 2.0 над stdio).
//! Два режима:
//!   • без флагов — standalone, своё ядро (для запуска отдельно от GUI);
//!   • --http <url> — прокси в HTTP API запущенного polygram.exe (единое ядро).

use polygram_api::ApiService;
use polygram_core::Command;
use serde_json::{json, Value};
use std::io::{self, BufRead, Read, Write};
use std::sync::mpsc;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

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
        Self { api, mcp_tx: None, http_base: None }
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
            "initialize" => self.handle_initialize(params),
            "tools/list" => self.handle_tools_list(),
            "tools/call" => self.handle_tools_call(params),
            _ => return Some(jsonrpc_error(id.as_ref(), -32601, "Method not found")),
        };

        Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result,
        }))
    }

    fn handle_initialize(&self, _params: Value) -> Value {
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "polygram",
                "version": "0.1.0"
            }
        })
    }

    fn handle_tools_list(&self) -> Value {
        json!({
            "tools": [
                {
                    "name": "execute_command",
                    "description": "Выполнить команду Polygram (JSON)",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "command": { "type": "string", "description": "JSON-команда" }
                        },
                        "required": ["command"]
                    }
                },
                {
                    "name": "get_snapshot",
                    "description": "Получить последний снимок состояния",
                    "inputSchema": { "type": "object", "properties": {} }
                },
                {
                    "name": "get_objects",
                    "description": "Список всех объектов",
                    "inputSchema": { "type": "object", "properties": {} }
                },
                {
                    "name": "get_stats",
                    "description": "Статистика графа",
                    "inputSchema": { "type": "object", "properties": {} }
                }
            ]
        })
    }

    fn handle_tools_call(&self, params: Value) -> Value {
        let name = match params.get("name").and_then(|v| v.as_str()) {
            Some(n) => n,
            None => return mcp_tool_error("Missing tool name"),
        };
        let args = params.get("arguments").cloned().unwrap_or(json!({}));

        match name {
            "execute_command" => {
                let cmd_json = match args.get("command").and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => return mcp_tool_error("Missing command argument"),
                };
                if let Some(base) = &self.http_base {
                    match proxy_post_command(base, cmd_json) {
                        Ok(body) => {
                            let cmd_str = cmd_json.to_owned();
                            if let Some(tx) = &self.mcp_tx {
                                let _ = tx.send(McpEvent { cmd: cmd_str, result: body.clone() });
                            }
                            mcp_tool_result(&body)
                        }
                        Err(e) => {
                            let err_str = format!("прокси HTTP: {e}");
                            if let Some(tx) = &self.mcp_tx {
                                let _ = tx.send(McpEvent { cmd: cmd_json.to_owned(), result: err_str.clone() });
                            }
                            mcp_tool_error(&err_str)
                        }
                    }
                } else {
                    let cmd: Command = match serde_json::from_str(cmd_json) {
                        Ok(c) => c,
                        Err(_) => return mcp_tool_error("Invalid command JSON"),
                    };
                    let cmd_str = serde_json::to_string(&cmd).unwrap_or_else(|_| cmd_json.to_owned());
                    match self.api.apply(cmd) {
                        Ok(applied) => {
                            let result = format!("OK ревизия {}", applied.rev);
                            if let Some(tx) = &self.mcp_tx {
                                let _ = tx.send(McpEvent { cmd: cmd_str, result });
                            }
                            mcp_tool_result(&serde_json::to_string(&json!({"ok": true, "rev": applied.rev})).unwrap())
                        }
                        Err(e) => {
                            let result = format!("Ошибка: {e:?}");
                            if let Some(tx) = &self.mcp_tx {
                                let _ = tx.send(McpEvent { cmd: cmd_str, result });
                            }
                            mcp_tool_error(&format!("{e:?}"))
                        }
                    }
                }
            }
            "get_snapshot" => {
                if let Some(base) = &self.http_base {
                    match proxy_get(base, "/api/snapshot") {
                        Ok(v) => mcp_tool_result(&serde_json::to_string(&v).unwrap()),
                        Err(e) => {
                            let err_str = format!("{e}");
                            mcp_tool_error(&err_str)
                        }
                    }
                } else {
                    let pub_ = self.api.snapshot();
                    mcp_tool_result(&serde_json::to_string(&json!({ "rev": pub_.rev, "dynamic": pub_.dynamic.as_ref().clone(), "geometry": pub_.geometry.as_ref().clone() })).unwrap())
                }
            }
            "get_objects" => {
                if let Some(base) = &self.http_base {
                    match proxy_get(base, "/api/objects") {
                        Ok(v) => mcp_tool_result(&serde_json::to_string(&v).unwrap()),
                        Err(e) => {
                            let err_str = format!("{e}");
                            mcp_tool_error(&err_str)
                        }
                    }
                } else {
                    let objects = self.api.objects();
                    mcp_tool_result(&serde_json::to_string(&json!({ "objects": objects })).unwrap())
                }
            }
            "get_stats" => {
                if let Some(base) = &self.http_base {
                    match proxy_get(base, "/api/stats") {
                        Ok(v) => mcp_tool_result(&serde_json::to_string(&v).unwrap()),
                        Err(e) => {
                            let err_str = format!("{e}");
                            mcp_tool_error(&err_str)
                        }
                    }
                } else {
                    let stats = self.api.stats();
                    mcp_tool_result(&serde_json::to_string(&json!({ "stats": stats })).unwrap())
                }
            }
            _ => mcp_tool_error("Unknown tool"),
        }
    }
}

fn mcp_tool_result(text: &str) -> Value {
    json!({
        "content": [
            {
                "type": "text",
                "text": text
            }
        ],
        "isError": false
    })
}

fn mcp_tool_error(text: &str) -> Value {
    json!({
        "content": [
            {
                "type": "text",
                "text": text
            }
        ],
        "isError": true
    })
}

fn jsonrpc_error(id: Option<&Value>, code: i32, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id.cloned(),
        "error": { "code": code, "message": message }
    })
}

/// POST команду в HTTP API polygram.exe.
fn proxy_post_command(base: &str, cmd_json: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Валидируем, что команда — корректный JSON.
    serde_json::from_str::<Value>(cmd_json)
        .map_err(|e| format!("неверный JSON команды: {e}"))?;
    let resp = ureq::post(&format!("{base}/api/commands"))
        .set("Content-Type", "application/json")
        .send_bytes(cmd_json.as_bytes())?;
    Ok(resp.into_string()?.trim().to_owned())
}

/// GET из HTTP API polygram.exe.
fn proxy_get(base: &str, path: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let resp = ureq::get(&format!("{base}{path}")).call()?;
    let body: Value = serde_json::from_str(&resp.into_string()?)?;
    Ok(body)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // --http <url> — прокси в запущенный polygram.exe (единое ядро).
    let http_base = if args.len() >= 3 && args[1] == "--http" {
        Some(args[2].clone())
    } else {
        None
    };

    if let Some(base) = &http_base {
        // Прокси-режим: нет своего ядра, все команды идут в polygram.exe.
        let api = ApiService::default(); // используется только для совместимости интерфейса
        let running = Arc::new(AtomicBool::new(true));
        McpServer::new(api).with_http(base.clone()).run(running);
    } else {
        // Standalone — своё ядро.
        let api = ApiService::default();
        let running = Arc::new(AtomicBool::new(true));
        McpServer::new(api).run(running);
    }
}

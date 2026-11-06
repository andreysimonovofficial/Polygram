//! MCP сервер Polygram (JSON-RPC 2.0 over stdio).

use polygram_core::Command;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

use super::ApiService;

pub struct McpServer {
    api: ApiService,
}

impl McpServer {
    pub fn new(api: ApiService) -> Self {
        Self { api }
    }

    pub fn run(self) {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut out = stdout.lock();

        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };
            if line.trim().is_empty() {
                continue;
            }
            let response = self.handle_message(&line);
            if let Some(resp) = response {
                let _ = writeln!(out, "{}", serde_json::to_string(&resp).unwrap());
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
            None => return jsonrpc_error(None, -32602, "Missing tool name"),
        };
        let args = params.get("arguments").cloned().unwrap_or(json!({}));

        match name {
            "execute_command" => {
                let cmd_json = match args.get("command").and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => return jsonrpc_error(None, -32602, "Missing command argument"),
                };
                let cmd: Command = match serde_json::from_str(cmd_json) {
                    Ok(c) => c,
                    Err(_) => return jsonrpc_error(None, -32602, "Invalid command JSON"),
                };
                match self.api.apply(cmd) {
                    Ok(applied) => json!({ "ok": true, "rev": applied.rev }),
                    Err(e) => json!({ "ok": false, "error": format!("{e:?}") }),
                }
            }
            "get_snapshot" => {
                let pub_ = self.api.snapshot();
                json!({ "rev": pub_.rev, "dynamic": pub_.dynamic.as_ref().clone(), "geometry": pub_.geometry.as_ref().clone() })
            }
            "get_objects" => {
                let objects = self.api.objects();
                json!({ "objects": objects })
            }
            "get_stats" => {
                let stats = self.api.stats();
                json!({ "stats": stats })
            }
            _ => jsonrpc_error(None, -32602, "Unknown tool"),
        }
    }
}

fn jsonrpc_error(id: Option<&Value>, code: i32, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id.cloned(),
        "error": { "code": code, "message": message }
    })
}

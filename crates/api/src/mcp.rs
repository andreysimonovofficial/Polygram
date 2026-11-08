//! MCP сервер Polygram (JSON-RPC 2.0 over stdio).

use polygram_core::Command;
use serde_json::{json, Value};
use std::io::{self, BufRead, Read, Write};
use std::sync::mpsc;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

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
}

impl McpServer {
    pub fn new(api: ApiService) -> Self {
        Self { api, mcp_tx: None }
    }

    pub fn with_events(mut self, tx: mpsc::Sender<McpEvent>) -> Self {
        self.mcp_tx = Some(tx);
        self
    }

    pub fn run(self, running: Arc<AtomicBool>) {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut out = stdout.lock();
        let mut stdin = stdin.lock();

        while running.load(Ordering::SeqCst) {
            // Читаем HTTP-подобные заголовки (Content-Length и т.д.) MCP stdio транспорта.
            let mut content_length: Option<usize> = None;
            loop {
                let mut line = String::new();
                match stdin.read_line(&mut line) {
                    Ok(0) => return, // EOF
                    Ok(_) => {}
                    Err(_) => return,
                }
                let line = line.trim_end_matches(&['\r', '\n'][..]);
                if line.is_empty() {
                    break; // Конец заголовков
                }
                if let Some(rest) = line.strip_prefix("Content-Length:") {
                    if let Ok(n) = rest.trim().parse::<usize>() {
                        content_length = Some(n);
                    }
                }
            }

            let n = match content_length {
                Some(n) => n,
                None => continue,
            };

            // Читаем тело JSON-RPC.
            let mut body = vec![0u8; n];
            if stdin.read_exact(&mut body).is_err() {
                break;
            }
            let line = match String::from_utf8(body) {
                Ok(s) => s,
                Err(_) => continue,
            };
            if line.trim().is_empty() {
                continue;
            }

            let response = self.handle_message(&line);
            if let Some(resp) = response {
                let resp_str = serde_json::to_string(&resp).unwrap();
                let resp_bytes = resp_str.as_bytes();
                let _ = write!(
                    out,
                    "Content-Length: {}\r\n\r\n{}",
                    resp_bytes.len(),
                    resp_str
                );
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
            None => return mcp_tool_error("Missing tool name"),
        };
        let args = params.get("arguments").cloned().unwrap_or(json!({}));

        match name {
            "execute_command" => {
                let cmd_json = match args.get("command").and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => return mcp_tool_error("Missing command argument"),
                };
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
                        mcp_tool_result(&format!("{}", serde_json::to_string(&json!({"ok": true, "rev": applied.rev})).unwrap()))
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
            "get_snapshot" => {
                let pub_ = self.api.snapshot();
                mcp_tool_result(&serde_json::to_string(&json!({ "rev": pub_.rev, "dynamic": pub_.dynamic.as_ref().clone(), "geometry": pub_.geometry.as_ref().clone() })).unwrap())
            }
            "get_objects" => {
                let objects = self.api.objects();
                mcp_tool_result(&serde_json::to_string(&json!({ "objects": objects })).unwrap())
            }
            "get_stats" => {
                let stats = self.api.stats();
                mcp_tool_result(&serde_json::to_string(&json!({ "stats": stats })).unwrap())
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

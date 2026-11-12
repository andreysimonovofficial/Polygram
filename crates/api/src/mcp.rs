//! MCP сервер Polygram (JSON-RPC 2.0 over stdio).

use polygram_core::Command;
use polygram_core::types::{MetricKind, NodeKind};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::sync::mpsc;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

use super::ApiService;

/// HTTP-прокси команды в запущенный polygram.exe.
fn proxy_post_command(base: &str, cmd_json: &str) -> Result<String, Box<dyn std::error::Error>> {
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
            },
            "instructions": "Polygram MCP Server — инструкция для агентов:\n\nПОРЯДОК РАБОТЫ:\n1. Сначала вызовите get_commands, чтобы увидеть полный список команд с примерами.\n2. Используйте отдельные инструменты (add_object, rename_object и т.д.) вместо execute_command — у них понятные поля.\n3. execute_command принимает произвольную JSON-команду в поле command.\n\nФОРМАТ КОМАНД:\n- Каждая команда — JSON с полем \"cmd\" (имя команды) и остальными полями в зависимости от типа.\n- Пример add_object: {\"cmd\":\"add_object\",\"name\":\"Claude\",\"kind\":\"process\"}\n\nИМЕНА:\n- Пока пишите имена объектов латиницей (поддержка кириллицы в разработке).\n- ID объекта можно не указывать — сгенерируется автоматически.\n- Позиция at=[x,y] можно не указывать — подберётся автоматически.\n\nОШИБКИ:\n- Если команда не принимается, проверьте поле cmd и обязательные поля через get_commands.\n- При неверном JSON или неизвестном cmd вернётся понятное сообщение со списком доступных команд."
        })
    }

    fn handle_tools_list(&self) -> Value {
        json!({
            "tools": [
                {
                    "name": "get_commands",
                    "description": "Справочник всех команд Polygram с описанием полей и примерами. Вызовите этот инструмент первым, чтобы узнать формат команд.",
                    "inputSchema": { "type": "object", "properties": {} }
                },
                {
                    "name": "get_snapshot",
                    "description": "Получить полный снимок состояния графа (объекты, метрики, связи).",
                    "inputSchema": { "type": "object", "properties": {} }
                },
                {
                    "name": "get_objects",
                    "description": "Список всех объектов на холсте.",
                    "inputSchema": { "type": "object", "properties": {} }
                },
                {
                    "name": "get_stats",
                    "description": "Статистика графа: количество объектов, связей, показателей.",
                    "inputSchema": { "type": "object", "properties": {} }
                },
                {
                    "name": "add_object",
                    "description": "Создать объект на холсте (процесс, отдел). Имя пишите латиницей.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string", "description": "Имя объекта (латиницей), например \"Claude\"" },
                            "kind": { "type": "string", "description": "Тип: process (процесс), department (отдел). По умолчанию process." },
                            "id": { "type": "string", "description": "Идентификатор объекта. Если не указан, сгенерируется автоматически." },
                            "at": { "type": "array", "description": "Позиция [x, y]. Если не указана, подбирается автоматически." },
                            "parent": { "type": "string", "description": "ID родительского отдела. null — корень." }
                        },
                        "required": ["name"]
                    }
                },
                {
                    "name": "rename_object",
                    "description": "Переименовать существующий объект.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "object": { "type": "string", "description": "ID объекта для переименования" },
                            "name": { "type": "string", "description": "Новое имя объекта (латиницей)" }
                        },
                        "required": ["object", "name"]
                    }
                },
                {
                    "name": "remove_object",
                    "description": "Удалить объект с холста.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "object": { "type": "string", "description": "ID объекта для удаления" }
                        },
                        "required": ["object"]
                    }
                },
                {
                    "name": "move_object",
                    "description": "Переместить объект на новые координаты.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "object": { "type": "string", "description": "ID объекта" },
                            "at": { "type": "array", "description": "Новая позиция [x, y]" }
                        },
                        "required": ["object", "at"]
                    }
                },
                {
                    "name": "add_metric_def",
                    "description": "Создать глобальный показатель (метрику).",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string", "description": "Имя показателя" },
                            "kind": { "type": "string", "description": "Тип: number, time, percentage, binary, duration, formula" },
                            "unit": { "type": "string", "description": "Единица измерения, например \"$\" или \"шт.\"" },
                            "id": { "type": "string", "description": "Идентификатор показателя. Если не указан, сгенерируется автоматически." }
                        },
                        "required": ["name", "kind"]
                    }
                },
                {
                    "name": "execute_command",
                    "description": "Выполнить произвольную команду Polygram в формате JSON. Доступные команды: add_object, rename_object, remove_object, move_object, add_metric_def, remove_metric_def, rename_metric_def, set_metric_def_unit, add_metric_entry, remove_metric_entry, set_value, set_required, set_max, set_fact, connect, disconnect, auto_layout, reroute_all. Каждая команда имеет поле cmd с именем команды и остальные поля в зависимости от типа. Пример add_object: {\"cmd\":\"add_object\",\"name\":\"Claude\",\"kind\":\"process\"}.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "command": { "type": "string", "description": "JSON-команда, например {\"cmd\":\"add_object\",\"name\":\"Claude\",\"kind\":\"process\"}" }
                        },
                        "required": ["command"]
                    }
                }
            ]
        })
    }

    fn handle_tools_call(&self, params: Value) -> Value {
        let name = match params.get("name").and_then(|v| v.as_str()) {
            Some(n) => n,
            None => return mcp_tool_error("Не указано имя инструмента. Доступные: get_commands, add_object, rename_object, remove_object, move_object, add_metric_def, get_snapshot, get_objects, get_stats, execute_command."),
        };
        let args = params.get("arguments").cloned().unwrap_or(json!({}));

        match name {
            "get_commands" => mcp_tool_result(&serde_json::to_string(&json!({
                "commands": [
                    {"cmd": "add_object", "example": "{\"cmd\":\"add_object\",\"name\":\"Claude\",\"kind\":\"process\"}", "description": "Создать объект (процесс/отдел)"},
                    {"cmd": "rename_object", "example": "{\"cmd\":\"rename_object\",\"object\":\"claude\",\"name\":\"Новое имя\"}", "description": "Переименовать объект"},
                    {"cmd": "remove_object", "example": "{\"cmd\":\"remove_object\",\"object\":\"claude\"}", "description": "Удалить объект"},
                    {"cmd": "move_object", "example": "{\"cmd\":\"move_object\",\"object\":\"claude\",\"at\":[100,200]}", "description": "Переместить объект"},
                    {"cmd": "add_metric_def", "example": "{\"cmd\":\"add_metric_def\",\"name\":\"Прибыль\",\"kind\":\"number\",\"unit\":\"$\"}", "description": "Создать показатель"},
                    {"cmd": "get_snapshot", "example": "{\"cmd\":\"get_snapshot\"}", "description": "Снимок состояния графа"},
                    {"cmd": "get_objects", "example": "{\"cmd\":\"get_objects\"}", "description": "Список объектов"},
                    {"cmd": "get_stats", "example": "{\"cmd\":\"get_stats\"}", "description": "Статистика графа"}
                ]
            })).unwrap()),
            "add_object" => {
                let name = match args.get("name").and_then(|v| v.as_str()) {
                    Some(n) => n.to_owned(),
                    None => return mcp_tool_error("Для add_object требуется поле name (имя объекта). Пример: {\"name\":\"Claude\",\"kind\":\"process\"}"),
                };
                let kind = args.get("kind").and_then(|v| v.as_str()).unwrap_or("process");
                let author = args.get("author").and_then(|v| v.as_str()).map(|s| s.to_owned());
                let cmd = Command::AddObject {
                    id: args.get("id").and_then(|v| v.as_str()).map(|s| s.to_owned()),
                    name,
                    kind: match kind {
                        "department" => NodeKind::Department,
                        _ => NodeKind::Process,
                    },
                    at: args.get("at").and_then(|v| v.as_array()).map(|arr| {
                        arr.iter().filter_map(|v| v.as_i64()).map(|v| v as i32).collect::<Vec<_>>().try_into().unwrap_or([0, 0])
                    }),
                    parent: args.get("parent").and_then(|v| v.as_str()).map(|s| s.to_owned()),
                    author,
                };
                self.apply_command(cmd, &args)
            }
            "rename_object" => {
                let object = match args.get("object").and_then(|v| v.as_str()) {
                    Some(s) => s.to_owned(),
                    None => return mcp_tool_error("Для rename_object требуется поле object (ID объекта)."),
                };
                let name = match args.get("name").and_then(|v| v.as_str()) {
                    Some(s) => s.to_owned(),
                    None => return mcp_tool_error("Для rename_object требуется поле name (новое имя)."),
                };
                let cmd = Command::RenameObject { object, name };
                self.apply_command(cmd, &args)
            }
            "remove_object" => {
                let object = match args.get("object").and_then(|v| v.as_str()) {
                    Some(s) => s.to_owned(),
                    None => return mcp_tool_error("Для remove_object требуется поле object (ID объекта)."),
                };
                let cmd = Command::RemoveObject { object };
                self.apply_command(cmd, &args)
            }
            "move_object" => {
                let object = match args.get("object").and_then(|v| v.as_str()) {
                    Some(s) => s.to_owned(),
                    None => return mcp_tool_error("Для move_object требуется поле object (ID объекта)."),
                };
                let at = match args.get("at").and_then(|v| v.as_array()) {
                    Some(arr) => {
                        let coords: Vec<i32> = arr.iter().filter_map(|v| v.as_i64()).map(|v| v as i32).collect();
                        if coords.len() != 2 {
                            return mcp_tool_error("Для move_object поле at должно быть массивом из двух чисел [x, y].");
                        }
                        [coords[0], coords[1]]
                    }
                    None => return mcp_tool_error("Для move_object требуется поле at ([x, y])."),
                };
                let cmd = Command::MoveObject { object, at };
                self.apply_command(cmd, &args)
            }
            "add_metric_def" => {
                let name = match args.get("name").and_then(|v| v.as_str()) {
                    Some(s) => s.to_owned(),
                    None => return mcp_tool_error("Для add_metric_def требуется поле name (имя показателя)."),
                };
                let kind = match args.get("kind").and_then(|v| v.as_str()) {
                    Some(s) => s.to_owned(),
                    None => return mcp_tool_error("Для add_metric_def требуется поле kind (number, time, date, deadline, percentage, binary, duration, money, text, formula)."),
                };
                let cmd = Command::AddMetricDef {
                    id: args.get("id").and_then(|v| v.as_str()).map(|s| s.to_owned()),
                    name,
                    kind: match kind.as_str() {
                        "time" => MetricKind::Time,
                        "date" => MetricKind::Date,
                        "deadline" => MetricKind::Deadline,
                        "percentage" => MetricKind::Percentage,
                        "binary" => MetricKind::Binary,
                        "duration" => MetricKind::Duration,
                        "money" => MetricKind::Money,
                        "text" => MetricKind::Text,
                        "formula" => MetricKind::Formula,
                        _ => MetricKind::Number,
                    },
                    unit: args.get("unit").and_then(|v| v.as_str()).unwrap_or("ед.").to_owned(),
                    api: None,
                };
                self.apply_command(cmd, &args)
            }
            "execute_command" => {
                let cmd_json = match args.get("command").and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => return mcp_tool_error("Для execute_command требуется поле command (JSON-команда). Доступные команды: add_object, rename_object, remove_object, move_object, add_metric_def и другие. См. get_commands."),
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
                        Err(e) => return mcp_tool_error(&format!("Не удалось распарсить JSON-команду: {e}. Пример: {{\"cmd\":\"add_object\",\"name\":\"Claude\",\"kind\":\"process\"}}. Доступные команды: add_object, rename_object, remove_object, move_object, add_metric_def. См. get_commands.")),
                    };
                    self.apply_command(cmd, &args)
                }
            }
            "get_snapshot" => {
                if let Some(base) = &self.http_base {
                    match proxy_get(base, "/api/snapshot") {
                        Ok(v) => mcp_tool_result(&serde_json::to_string(&v).unwrap()),
                        Err(e) => mcp_tool_error(&format!("{e}"))
                    }
                } else {
                    let pub_ = self.api.snapshot();
                    mcp_tool_result(&serde_json::to_string(&json!({ "rev": pub_.rev, "dynamic": pub_.dynamic.clone(), "geometry": pub_.geometry.clone() })).unwrap())
                }
            }
            "get_objects" => {
                if let Some(base) = &self.http_base {
                    match proxy_get(base, "/api/objects") {
                        Ok(v) => mcp_tool_result(&serde_json::to_string(&v).unwrap()),
                        Err(e) => mcp_tool_error(&format!("{e}"))
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
                        Err(e) => mcp_tool_error(&format!("{e}"))
                    }
                } else {
                    let stats = self.api.stats();
                    mcp_tool_result(&serde_json::to_string(&json!({ "stats": stats })).unwrap())
                }
            }
            _ => mcp_tool_error(&format!("Неизвестный инструмент \"{name}\". Доступные: get_commands, add_object, rename_object, remove_object, move_object, add_metric_def, get_snapshot, get_objects, get_stats, execute_command.")),
        }
    }

    fn apply_command(&self, cmd: Command, _args: &Value) -> Value {
        if let Some(base) = &self.http_base {
            let cmd_json = serde_json::to_string(&cmd).unwrap();
            match proxy_post_command(base, &cmd_json) {
                Ok(body) => {
                    let cmd_str = cmd_json;
                    if let Some(tx) = &self.mcp_tx {
                        let _ = tx.send(McpEvent { cmd: cmd_str, result: body.clone() });
                    }
                    mcp_tool_result(&body)
                }
                Err(e) => {
                    let err_str = format!("прокси HTTP: {e}");
                    if let Some(tx) = &self.mcp_tx {
                        let _ = tx.send(McpEvent { cmd: cmd_json, result: err_str.clone() });
                    }
                    mcp_tool_error(&err_str)
                }
            }
        } else {
            let cmd_str = serde_json::to_string(&cmd).unwrap();
            match self.api.apply(cmd) {
                Ok(applied) => {
                    if let Some(tx) = &self.mcp_tx {
                        let _ = tx.send(McpEvent { cmd: cmd_str, result: format!("OK ревизия {}", applied.rev) });
                    }
                    mcp_tool_result(&serde_json::to_string(&json!({"ok": true, "rev": applied.rev})).unwrap())
                }
                Err(e) => {
                    let err_str = format!("{e:?}");
                    if let Some(tx) = &self.mcp_tx {
                        let _ = tx.send(McpEvent { cmd: cmd_str, result: err_str.clone() });
                    }
                    mcp_tool_error(&err_str)
                }
            }
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

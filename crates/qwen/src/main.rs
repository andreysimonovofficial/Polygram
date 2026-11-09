//! Локальный Qwen-агент Polygram (отдельный процесс, не внутри GUI).
//! Читает общий чат через HTTP API, отвечает через Ollama, создаёт объекты через MCP-инструмент add_object.
//! Запускается вручную: `cargo run -p polygram-qwen`. Требует запущенного polygram-api (8080);
//! Ollama (11434) опционален — при недоступности агент отвечает шаблоном.

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const API_BASE: &str = "http://127.0.0.1:8080";
const OLLAMA_BASE: &str = "http://localhost:11434";
const OLLAMA_MODEL: &str = "qwen2.5";
const AGENT_AUTHOR: &str = "qwen";

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ChatObject {
    id: String,
    name: String,
    kind: String,
    at: [i32; 2],
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ChatMessage {
    id: u64,
    author: String,
    text: String,
    #[serde(default)]
    reply_to: Option<String>,
    pub ts: u64,
    #[serde(default)]
    object: Option<ChatObject>,
}

fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn exe_dir() -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
}

fn mcp_exe_path() -> std::path::PathBuf {
    exe_dir().join("polygram-mcp.exe")
}

/// Минимальный MCP-клиент (stdio, построчный JSON-RPC 2.0) к polygram-mcp --http.
struct McpClient {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    stdout: std::io::BufReader<std::process::ChildStdout>,
    next_id: u64,
}

impl McpClient {
    fn spawn() -> Result<Self, String> {
        let mcp_exe = mcp_exe_path();
        let mut child = Command::new(&mcp_exe)
            .args(["--http", API_BASE])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| format!("не удалось запустить {}: {e}", mcp_exe.display()))?;
        let stdin = child.stdin.take().ok_or("MCP: нет stdin")?;
        let stdout = std::io::BufReader::new(child.stdout.take().ok_or("MCP: нет stdout")?);
        Ok(Self {
            child,
            stdin,
            stdout,
            next_id: 0,
        })
    }

    fn notify(&mut self, method: &str, params: Value) -> Result<(), String> {
        let msg = json!({"jsonrpc":"2.0","method":method,"params":params});
        let line = serde_json::to_string(&msg).unwrap();
        writeln!(self.stdin, "{line}").map_err(|e| e.to_string())?;
        self.stdin.flush().map_err(|e| e.to_string())
    }

    fn request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        self.next_id += 1;
        let id = self.next_id;
        let msg = json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
        let line = serde_json::to_string(&msg).unwrap();
        writeln!(self.stdin, "{line}").map_err(|e| e.to_string())?;
        self.stdin.flush().map_err(|e| e.to_string())?;
        loop {
            let mut raw = String::new();
            self.stdout.read_line(&mut raw).map_err(|e| e.to_string())?;
            if raw.trim().is_empty() {
                continue;
            }
            let resp: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
            if resp.get("id").and_then(|v| v.as_u64()) == Some(id) {
                return Ok(resp);
            }
        }
    }

    fn add_object(&mut self, name: &str) -> Result<Value, String> {
        self.request(
            "tools/call",
            json!({
                "name": "add_object",
                "arguments": {
                    "name": name,
                    "kind": "process",
                    "author": AGENT_AUTHOR,
                },
            }),
        )
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

fn get_chat_messages(client: &Client) -> Vec<ChatMessage> {
    match client.get(format!("{API_BASE}/api/chat/messages")).send() {
        Ok(resp) if resp.status().is_success() => match resp.json::<Vec<ChatMessage>>() {
            Ok(m) => m,
            Err(e) => {
                eprintln!("qwen: не удалось разобрать ленту чата: {e}");
                Vec::new()
            }
        },
        Ok(resp) => {
            eprintln!("qwen: чат вернул HTTP {}", resp.status());
            Vec::new()
        }
        Err(e) => {
            eprintln!("qwen: чат недоступен ({e})");
            Vec::new()
        }
    }
}

fn post_chat_message(client: &Client, author: &str, text: &str, reply_to: Option<&str>) {
    let body = json!({
        "author": author,
        "text": text,
        "reply_to": reply_to,
    });
    match client
        .post(format!("{API_BASE}/api/chat/messages"))
        .json(&body)
        .send()
    {
        Ok(resp) if resp.status().is_success() => {}
        Ok(resp) => eprintln!("qwen: не удалось отправить сообщение: HTTP {}", resp.status()),
        Err(e) => eprintln!("qwen: ошибка отправки сообщения: {e}"),
    }
}

fn ollama_reply(client: &Client, user_text: &str) -> Option<String> {
    let system = "Ты — локальная модель Qwen в Polygram. Отвечай по-русски, кратко и по существу.\
 Если пользователь просит создать объект на холсте, сообщи об этом словами «создай объект».\
 Ты можешь создавать процессы на холсте и отвечать в общий чат.";
    let body = json!({
        "model": OLLAMA_MODEL,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user_text},
        ],
        "stream": false,
        "options": {"temperature": 0.7},
    });
    match client.post(format!("{OLLAMA_BASE}/api/chat")).json(&body).send() {
        Ok(resp) if resp.status().is_success() => match resp.json::<Value>() {
            Ok(v) => v
                .get("message")
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_str())
                .map(|s| s.to_owned()),
            Err(e) => {
                eprintln!("qwen: не удалось разобрать ответ Ollama: {e}");
                None
            }
        },
        Ok(resp) => {
            eprintln!("qwen: Ollama вернул HTTP {}", resp.status());
            None
        }
        Err(e) => {
            eprintln!("qwen: Ollama недоступен ({e})");
            None
        }
    }
}

fn wants_create_object(text: &str) -> bool {
    let t = text.to_lowercase();
    t.contains("создай") || t.contains("create") || t.contains("объект") || t.contains("объекта")
}

fn main() {
    let client = Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .expect("не удалось создать HTTP-клиент");
    eprintln!("qwen-агент запущен (автор {AGENT_AUTHOR}), API {API_BASE}");

    let mut mcp = match McpClient::spawn() {
        Ok(c) => {
            eprintln!("qwen: MCP-клиент подключён к polygram-mcp");
            Some(c)
        }
        Err(e) => {
            eprintln!("qwen: MCP не запущен ({e}), создание объектов через HTTP будет недоступно");
            None
        }
    };

    if let Some(mcp) = mcp.as_mut() {
        let _ = mcp.notify("initialized", json!({}));
        let _ = mcp.request("initialize", json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "qwen-agent", "version": "0.1.0"}
        }));
    }

    let mut last_id: u64 = 0;

    loop {
        let messages = get_chat_messages(&client);
        let mut new_msgs: Vec<&ChatMessage> = Vec::new();
        for m in &messages {
            if m.id > last_id {
                new_msgs.push(m);
            }
        }
        if let Some(max) = messages.iter().map(|m| m.id).max() {
            last_id = max;
        }

        for m in new_msgs {
            if m.author == AGENT_AUTHOR {
                continue;
            }
            // Реагируем на сообщения пользователя (и на ответы нам).
            let addressed = m.reply_to.as_deref() == Some(AGENT_AUTHOR) || m.author == "user" || m.author == "http";
            if !addressed {
                continue;
            }

            eprintln!("qwen: новое сообщение от {}: {}", m.author, m.text);

            // Попытка ответа через Ollama.
            let reply = ollama_reply(&client, &m.text).unwrap_or_else(|| {
                if wants_create_object(&m.text) {
                    format!("Принял, {} — создаю объект на холсте.", m.author)
                } else {
                    format!(
                        "{} написал: «{}». (Ollama недоступен — отвечаю шаблоном.)",
                        m.author, m.text
                    )
                }
            });

            // Создать объект, если просили.
            if wants_create_object(&m.text) {
                if let Some(mcp) = mcp.as_mut() {
                    let name = format!("Qwen-{}", m.text.split_whitespace().next().unwrap_or("объект"));
                    match mcp.add_object(&name) {
                        Ok(_) => eprintln!("qwen: объект «{name}» создан через MCP"),
                        Err(e) => eprintln!("qwen: ошибка создания объекта через MCP: {e}"),
                    }
                }
            }

            post_chat_message(&client, AGENT_AUTHOR, &reply, None);
        }

        thread::sleep(Duration::from_secs(2));
    }
}

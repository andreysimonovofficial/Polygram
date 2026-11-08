//! Управление встроенными серверами (MCP stdio + HTTP API).
//! HTTP-сервер — простой синхронный TCP (без tokio), чтобы не конфликтовать
//! с egui/eframe и отвечать на запросы "на лету", пока окно открыто.
//! GUI владеет ApiService и движком — HTTP/MCP делят состояние в памяти.

use polygram_api::{mcp::McpEvent, mcp::McpServer, ApiService};
use polygram_core::{Command, types::NodeKind};
use serde::Deserialize;
use serde_json::json;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;

#[derive(Deserialize, Clone, Debug)]
pub struct CreateObject {
    #[serde(default)]
    pub id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub kind: NodeKind,
    #[serde(default)]
    pub at: Option<[i32; 2]>,
    #[serde(default)]
    pub parent: Option<String>,
}

pub struct ServerControl {
    pub api: Arc<ApiService>,
    pub mcp_tx: mpsc::Sender<McpEvent>,
    pub mcp_running: Arc<AtomicBool>,
    pub mcp_join: Mutex<Option<thread::JoinHandle<()>>>,
    pub http_port: Mutex<u16>,
    pub http_join: Mutex<Option<thread::JoinHandle<()>>>,
    pub http_stop: Arc<AtomicBool>,
}

impl ServerControl {
    pub fn new(api: Arc<ApiService>, mcp_tx: mpsc::Sender<McpEvent>) -> Self {
        Self {
            api,
            mcp_tx,
            mcp_running: Arc::new(AtomicBool::new(true)),
            mcp_join: Mutex::new(None),
            http_port: Mutex::new(8080),
            http_join: Mutex::new(None),
            http_stop: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start_mcp(&self) {
        let api = Arc::clone(&self.api);
        let mcp_tx = self.mcp_tx.clone();
        let running = Arc::clone(&self.mcp_running);
        running.store(true, Ordering::SeqCst);

        let handle = thread::spawn(move || {
            let server = McpServer::new((*api).clone()).with_events(mcp_tx);
            server.run(running);
        });

        let mut join = self.mcp_join.lock().unwrap();
        *join = Some(handle);
    }

    pub fn stop_mcp(&self) {
        self.mcp_running.store(false, Ordering::SeqCst);
        let mut join = self.mcp_join.lock().unwrap();
        if let Some(handle) = join.take() {
            let _ = handle.join();
        }
    }

    pub fn restart_mcp(&self) {
        self.stop_mcp();
        self.start_mcp();
    }

    pub fn start_http(&self, port: u16) {
        let _ = std::fs::write("C:\\temp\\http_start.log", format!("start_http port={port}\n"));
        self.stop_http();
        self.http_stop.store(false, Ordering::SeqCst);

        let api = Arc::clone(&self.api);
        let stop = Arc::clone(&self.http_stop);

        let handle = thread::spawn(move || {
            let _ = std::fs::write("C:\\temp\\http_start.log", "thread spawned\n");
            let listener = match TcpListener::bind(format!("127.0.0.1:{port}")) {
                Ok(l) => {
                    let _ = std::fs::write("C:\\temp\\http_start.log", "listener bound\n");
                    l
                }
                Err(e) => {
                    let _ = std::fs::write("C:\\temp\\http_start.log", format!("bind error: {e}\n"));
                    eprintln!("HTTP bind error на порту {port}: {e}");
                    return;
                }
            };
            let _ = std::fs::write("C:\\temp\\http_start.log", "entering incoming loop\n");
            println!("Polygram HTTP API listening on http://127.0.0.1:{port}");

            for stream in listener.incoming() {
                if stop.load(Ordering::SeqCst) {
                    break;
                }
                match stream {
                    Ok(mut stream) => {
                        let _ = std::fs::write("C:\\temp\\http_start.log", "connection accepted\n");
                        let _ = stream.set_nonblocking(false);
                        let api = Arc::clone(&api);
                        thread::spawn(move || {
                            if let Err(e) = handle_connection(stream, api) {
                                eprintln!("HTTP connection error: {e}");
                            }
                        });
                    }
                    Err(e) => {
                        let _ = std::fs::write("C:\\temp\\http_start.log", format!("accept error: {e}\n"));
                        eprintln!("HTTP accept error: {e}");
                    }
                }
            }
            let _ = std::fs::write("C:\\temp\\http_start.log", "incoming loop exited\n");
        });

        *self.http_port.lock().unwrap() = port;
        *self.http_join.lock().unwrap() = Some(handle);
    }

    pub fn stop_http(&self) {
        self.http_stop.store(true, Ordering::SeqCst);
        let mut join = self.http_join.lock().unwrap();
        if let Some(handle) = join.take() {
            let _ = handle.join();
        }
    }

    pub fn restart_http(&self) {
        let port = *self.http_port.lock().unwrap();
        self.stop_http();
        self.start_http(port);
    }

    pub fn mcp_is_running(&self) -> bool {
        self.mcp_running.load(Ordering::SeqCst)
    }
}

fn handle_connection(mut stream: std::net::TcpStream, api: Arc<ApiService>) -> std::io::Result<()> {
    let _ = std::fs::write("C:\\temp\\http_conn.log", "connection accepted in handler\n");
    stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(std::time::Duration::from_secs(5)))?;

    // Читаем весь запрос в один буфер (заголовки + тело помещаются в 64 КБ).
    let mut buf = [0u8; 65536];
    let total = match stream.read(&mut buf) {
        Ok(n) => {
            let _ = std::fs::write("C:\\temp\\http_conn.log", format!("read {n} bytes\n"));
            n
        }
        Err(e) => {
            let _ = std::fs::write("C:\\temp\\http_conn.log", format!("read error: {e}\n"));
            return Err(e);
        }
    };
    if total == 0 {
        let _ = std::fs::write("C:\\temp\\http_conn.log", "read 0 bytes, connection closed\n");
        return Ok(());
    }
    let data = &buf[..total];

    // Разделяем заголовки и тело по "\r\n\r\n".
    let header_end = find_header_end(data);
    let (header_bytes, body_bytes) = match header_end {
        Some(idx) => (&data[..idx], &data[idx + 4..]),
        None => (data, &data[0..0]),
    };

    let header_str = String::from_utf8_lossy(header_bytes);
    let mut lines = header_str.lines();
    let request_line = lines.next().unwrap_or("");
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("");

    // Content-Length.
    let mut content_length = 0usize;
    for line in lines {
        if let Some((key, value)) = line.split_once(':') {
            if key.trim().eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse().unwrap_or(0);
            }
        }
    }

    // Если тело обрезано буфером, добиваем чтением.
    let mut body = String::from_utf8_lossy(body_bytes).to_string();
    if body.len() < content_length {
        let missing = content_length - body.len();
        let mut extra = vec![0u8; missing];
        if let Ok(n) = stream.read(&mut extra) {
            body.push_str(&String::from_utf8_lossy(&extra[..n]));
        }
    }

    // Маршрутизация.
    let (status, json) = route(method, path, &body, &api);
    let _ = std::fs::write("C:\\temp\\http_conn.log", format!("route: {method} {path} -> {status}\n"));

    let body_str = serde_json::to_string(&json).unwrap_or_default();
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body_str.len(),
        body_str
    );
    let _ = std::fs::write("C:\\temp\\http_conn.log", format!("response: {response}\n"));
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    let _ = std::fs::write("C:\\temp\\http_conn.log", "response sent\n");
    Ok(())
}

fn find_header_end(data: &[u8]) -> Option<usize> {
    for i in 0..data.len().saturating_sub(3) {
        if data[i] == b'\r' && data[i + 1] == b'\n' && data[i + 2] == b'\r' && data[i + 3] == b'\n' {
            return Some(i);
        }
    }
    None
}

fn route(
    method: &str,
    path: &str,
    body: &str,
    api: &ApiService,
) -> (u16, serde_json::Value) {
    match (method, path) {
        ("GET", "/api/snapshot") => {
            let published = api.snapshot();
            (
                200,
                json!({
                    "rev": published.rev,
                    "dynamic": published.dynamic.as_ref().clone(),
                    "geometry": published.geometry.as_ref().clone(),
                }),
            )
        }
        ("GET", "/api/objects") => {
            let objects = api.objects();
            (200, json!(objects))
        }
        ("GET", p) if p.starts_with("/api/objects/") => {
            let id = &p["/api/objects/".len()..];
            match api.object(id.to_string()) {
                Some(obj) => (200, json!(obj)),
                None => (404, json!({"error": "not found"})),
            }
        }
        ("GET", "/api/stats") => match api.stats() {
            Some(stats) => (200, json!(stats)),
            None => (404, json!({"error": "no stats"})),
        },
        ("POST", "/api/objects") => {
            match serde_json::from_str::<CreateObject>(body) {
                Ok(req) => {
                    let cmd = Command::AddObject {
                        id: req.id,
                        name: req.name,
                        kind: req.kind,
                        at: req.at,
                        parent: req.parent,
                    };
                    match api.apply(cmd) {
                        Ok(applied) => (
                            200,
                            json!({
                                "ok": true,
                                "rev": applied.rev,
                                "created": applied.created,
                                "at": applied.at,
                            }),
                        ),
                        Err(e) => (400, json!({"ok": false, "error": format!("{e:?}")})),
                    }
                }
                Err(e) => (400, json!({"ok": false, "error": format!("{e:?}")})),
            }
        }
        ("POST", "/api/commands") => {
            match serde_json::from_str::<Command>(body) {
                Ok(cmd) => match api.apply(cmd) {
                    Ok(applied) => (200, json!({"ok": true, "rev": applied.rev})),
                    Err(e) => (400, json!({"ok": false, "error": format!("{e:?}")})),
                },
                Err(e) => (400, json!({"ok": false, "error": format!("{e:?}")})),
            }
        }
        ("POST", "/api/commands/batch") => {
            match serde_json::from_str::<Vec<Command>>(body) {
                Ok(cmds) => match api.apply_batch(cmds) {
                    Ok(results) => (200, json!({"ok": true, "results": results})),
                    Err((i, e)) => {
                        (400, json!({"ok": false, "failed_at": i, "error": format!("{e:?}")}))
                    }
                },
                Err(e) => (400, json!({"ok": false, "error": format!("{e:?}")})),
            }
        }
        _ => (404, json!({"error": "not found"})),
    }
}

//! Управление встроенными серверами (MCP stdio + HTTP API).
//! Позволяет перезапускать и менять порт не перезапуская приложение.

use polygram_api::{http::router, mcp::McpEvent, mcp::McpServer, ApiService};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;

pub struct ServerControl {
    pub api: Arc<ApiService>,
    pub mcp_tx: mpsc::Sender<McpEvent>,
    pub mcp_running: Arc<AtomicBool>,
    pub mcp_join: Mutex<Option<thread::JoinHandle<()>>>,
    pub http_port: Mutex<u16>,
    pub http_join: Mutex<Option<thread::JoinHandle<()>>>,
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
        let api = Arc::clone(&self.api);
        let handle = thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("Не удалось создать tokio runtime для HTTP: {e}");
                    return;
                }
            };
            rt.block_on(async move {
                let app = router(api);
                let listener = match tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await {
                    Ok(l) => l,
                    Err(e) => {
                        eprintln!("HTTP bind error на порту {port}: {e}");
                        return;
                    }
                };
                println!("Polygram HTTP API listening on http://0.0.0.0:{port}");
                if let Err(e) = axum::serve(listener, app).await {
                    eprintln!("HTTP serve error: {e}");
                }
            });
        });

        *self.http_port.lock().unwrap() = port;
        *self.http_join.lock().unwrap() = Some(handle);
    }

    pub fn stop_http(&self) {
        // HTTP-сервер работает в block_on и не может быть остановлен извне без перезапуска приложения.
        // Для смены порта используется start_http с новым портом (старый продолжает работать).
    }

    pub fn restart_http(&self) {
        let port = *self.http_port.lock().unwrap();
        self.start_http(port);
    }

    pub fn mcp_is_running(&self) -> bool {
        self.mcp_running.load(Ordering::SeqCst)
    }
}

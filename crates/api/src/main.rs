//! Точка входа API-сервера Polygram.

use polygram_api::http::router;
use polygram_api::mcp::McpServer;
use polygram_api::ApiService;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let api = Arc::new(ApiService::default());

    // MCP сервер в отдельном потоке (stdio)
    let mcp_api = Arc::clone(&api);
    let running = Arc::new(AtomicBool::new(true));
    std::thread::spawn(move || {
        McpServer::new((*mcp_api).clone()).run(Arc::clone(&running));
    });

    // HTTP сервер
    let app = router(api);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    println!("Polygram API listening on http://0.0.0.0:8080");
    axum::serve(listener, app).await.unwrap();
}

//! Точка входа MCP-сервера Polygram (только stdio, без HTTP).
//! Подходит для локального подключения из Claude Desktop, Open WebUI и других MCP-клиентов.

use polygram_api::{mcp::McpServer, ApiService};

fn main() {
    let api = ApiService::default();
    McpServer::new(api).run();
}

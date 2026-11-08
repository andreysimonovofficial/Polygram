//! Точка входа API-сервера Polygram.
//! HTTP-сервер работает 24/7. Админ-команды через каналы.
//! GUI может пересобираться и перезапускаться сколько угодно — API сервер остаётся 24/7.

use polygram_api::http::router;
use polygram_api::mcp::McpServer;
use polygram_api::{admin_channels, ApiService};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let auto_save_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
        .join("polygram_auto.pg");
    let api = if auto_save_path.exists() {
        match ApiService::load_from_file(&auto_save_path) {
            Ok(api) => {
                eprintln!("Загружено автосохранение: {}", auto_save_path.display());
                api
            }
            Err(e) => {
                eprintln!("Не удалось загрузить автосохранение: {e}, создаём новый граф");
                ApiService::default()
            }
        }
    } else {
        ApiService::default()
    };
    let api = Arc::new(api);
    api.set_auto_save_path(auto_save_path.clone());

    // Каналы управления сервером.
    let (admin_tx, mut admin_rx) = admin_channels();
    api.set_admin_tx(admin_tx);

    // MCP сервер в отдельном потоке (stdio).
    let mcp_api = Arc::clone(&api);
    let running = Arc::new(AtomicBool::new(true));
    std::thread::spawn(move || {
        McpServer::new((*mcp_api).clone()).run(Arc::clone(&running));
    });

    // Флаги управления.
    let restart_flag = Arc::new(AtomicBool::new(false));
    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let port_cell = Arc::new(std::sync::Mutex::new(8080u16));

    let mut port: u16 = 8080;

    loop {
        let app = router(api.clone());
        let listener = match tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("HTTP bind error на порту {port}: {e}");
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                continue;
            }
        };
        println!("Polygram API listening on http://0.0.0.0:{port}");

        let server_handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        // Ждём админ-команду.
        tokio::select! {
            _ = admin_rx.restart.recv() => {
                eprintln!("Получена команда перезапуска сервера на порту {port}");
            }
            Some(new_port) = admin_rx.change_port.recv() => {
                eprintln!("Смена порта с {port} на {new_port}");
                *port_cell.lock().unwrap() = new_port;
            }
            _ = admin_rx.shutdown.recv() => {
                eprintln!("Получена команда остановки сервера");
                shutdown_flag.store(true, Ordering::SeqCst);
            }
        }

        // Останавливаем серверную задачу.
        server_handle.abort();
        let _ = server_handle.await;

        port = *port_cell.lock().unwrap();
        restart_flag.store(false, Ordering::SeqCst);

        if shutdown_flag.load(Ordering::SeqCst) {
            println!("Сервер остановлен");
            return;
        }
    }
}

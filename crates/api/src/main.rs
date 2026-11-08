//! Точка входа API-сервера Polygram.
//! HTTP-сервер работает 24/7 на порту 8080.
//! Админ-команды (restart_server, change_port, shutdown) через каналы.
//! GUI может пересобираться и перезапускаться сколько угодно — API сервер остаётся 24/7.

use polygram_api::http::router;
use polygram_api::mcp::McpServer;
use polygram_api::{admin_channels, ApiService};
use std::future::IntoFuture;
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

    // Флаги управления.
    let restart_flag = Arc::new(AtomicBool::new(false));
    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let port_cell = Arc::new(std::sync::Mutex::new(8080u16));

    // Фоновый монитор админ-каналов.
    {
        let restart_flag_m = Arc::clone(&restart_flag);
        let shutdown_flag_m = Arc::clone(&shutdown_flag);
        let port_cell_m = Arc::clone(&port_cell);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = admin_rx.restart.recv() => {
                        restart_flag_m.store(true, Ordering::SeqCst);
                        eprintln!("Получена команда перезапуска сервера");
                    }
                    Some(new_port) = admin_rx.change_port.recv() => {
                        *port_cell_m.lock().unwrap() = new_port;
                        eprintln!("Смена порта на {new_port}");
                    }
                    _ = admin_rx.shutdown.recv() => {
                        shutdown_flag_m.store(true, Ordering::SeqCst);
                        eprintln!("Получена команда остановки сервера");
                    }
                }
            }
        });
    }

    // MCP сервер в отдельном потоке (stdio).
    let mcp_api = Arc::clone(&api);
    let running = Arc::new(AtomicBool::new(true));
    std::thread::spawn(move || {
        McpServer::new((*mcp_api).clone()).run(Arc::clone(&running));
    });

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

        let serve_future = axum::serve(listener, app).into_future();
        tokio::pin!(serve_future);

        let restart_flag_s = Arc::clone(&restart_flag);
        let shutdown_flag_s = Arc::clone(&shutdown_flag);
        let port_cell_s = Arc::clone(&port_cell);
        let current_port = port;

        let shutdown_future = async move {
            loop {
                if shutdown_flag_s.load(Ordering::SeqCst) {
                    return;
                }
                if restart_flag_s.load(Ordering::SeqCst) {
                    restart_flag_s.store(false, Ordering::SeqCst);
                    return;
                }
                let new_port = *port_cell_s.lock().unwrap();
                if new_port != current_port {
                    return;
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        };

        tokio::select! {
            _ = &mut serve_future => {
                eprintln!("HTTP сервер завершён на порту {port}");
            }
            _ = shutdown_future => {
                eprintln!("Сигнал остановки/перезапуска сервера на порту {port}");
            }
        }

        port = *port_cell.lock().unwrap();
        restart_flag.store(false, Ordering::SeqCst);

        if shutdown_flag.load(Ordering::SeqCst) {
            println!("Сервер остановлен");
            return;
        }
    }
}

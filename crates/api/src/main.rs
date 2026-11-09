use axum::Router;
use polygram_api::{ApiService, http::router};
use std::sync::Arc;
use tokio::net::TcpListener;

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

    let app: Router = router(api.clone());

    let listener = match TcpListener::bind("0.0.0.0:8080").await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("HTTP bind error на порту 8080: {e}");
            std::process::exit(1);
        }
    };
    println!("Polygram API listening on http://0.0.0.0:8080");

    axum::serve(listener, app).await.unwrap();
}

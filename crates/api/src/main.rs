use axum::Router;
use polygram_api::{ApiService, backup_path, http::router};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;

fn try_load_with_backups(path: &PathBuf) -> Option<polygram_api::ApiService> {
    // Пробуем основной файл.
    if let Ok(api) = ApiService::load_from_file(path) {
        return Some(api);
    }
    // Пробуем резервные копии от самой свежей к самой старой.
    for i in 1..=polygram_api::MAX_BACKUPS {
        let bp = backup_path(path, i);
        if bp.exists() {
            if let Ok(api) = ApiService::load_from_file(&bp) {
                eprintln!("Восстановлено из резервной копии: {}", bp.display());
                return Some(api);
            }
        }
    }
    None
}

#[tokio::main]
async fn main() {
    let auto_save_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
        .join("polygram_auto.pg");
    let api = if auto_save_path.exists() || (1..=polygram_api::MAX_BACKUPS).any(|i| backup_path(&auto_save_path, i).exists()) {
        match try_load_with_backups(&auto_save_path) {
            Some(api) => {
                eprintln!("Загружено автосохранение: {}", auto_save_path.display());
                api.set_auto_save_path(auto_save_path.clone());
                api
            }
            None => {
                eprintln!("Не удалось загрузить автосохранение, создаём новый граф");
                let api = ApiService::default();
                api.set_auto_save_path(auto_save_path.clone());
                api
            }
        }
    } else {
        let api = ApiService::default();
        api.set_auto_save_path(auto_save_path.clone());
        api
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

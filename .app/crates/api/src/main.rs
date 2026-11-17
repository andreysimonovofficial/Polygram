use axum::Router;
use polygram_api::{ApiService, backup_path, http::router, storage::{auto_save_path, ensure_data_dir}};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::time::{self, Duration};

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
    // Директория данных в %LOCALAPPDATA%\Polygram\ (или ~/.config/polygram/).
    if let Err(e) = ensure_data_dir() {
        eprintln!("Не удалось создать директорию данных: {e}");
    }
    let auto_save_path = auto_save_path();
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

    // Фоновое автосохранение каждые 5 минут.
    let api_clone = Arc::clone(&api);
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(300));
        loop {
            interval.tick().await;
            let _ = api_clone.maybe_save();
            eprintln!("Плановое автосохранение выполнено.");
        }
    });

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

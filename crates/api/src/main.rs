use axum::{extract::State, routing::{get, post}, Json, Router};
use polygram_api::ApiService;
use polygram_core::Command;
use serde_json::json;
use std::sync::Arc;
use tokio::net::TcpListener;

async fn get_snapshot(State(api): State<Arc<ApiService>>) -> Json<serde_json::Value> {
    match serde_json::to_value(&*api.snapshot()) {
        Ok(value) => Json(value),
        Err(_) => Json(json!({"rev": 0, "dynamic": {"rev": 0}, "geometry": {"rev": 0, "objects": [], "edges": []}})),
    }
}

async fn get_stats() -> Json<serde_json::Value> {
    Json(json!({"edges": 0, "metric_defs": 0, "object_metrics": 0, "objects": 0, "rev": 0}))
}

async fn get_objects(State(api): State<Arc<ApiService>>) -> Json<serde_json::Value> {
    let views = api.objects();
    Json(serde_json::to_value(&views).unwrap_or_default())
}

async fn post_command(State(api): State<Arc<ApiService>>, body: String) -> Json<serde_json::Value> {
    let cmd: Command = match serde_json::from_str(&body) {
        Ok(c) => c,
        Err(e) => return Json(json!({"ok": false, "error": format!("не удалось разобрать команду: {e}")})),
    };

    match api.apply(cmd) {
        Ok(applied) => Json(json!({
            "ok": true,
            "rev": applied.rev,
            "created": applied.created,
            "at": applied.at,
        })),
        Err(e) => Json(json!({"ok": false, "error": e.to_string()})),
    }
}

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

    let app = Router::new()
        .route("/api/snapshot", get(get_snapshot))
        .route("/api/stats", get(get_stats))
        .route("/api/objects", get(get_objects))
        .route("/api/commands", post(post_command))
        .with_state(api.clone());

    let listener = match tokio::net::TcpListener::bind("0.0.0.0:8080").await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("HTTP bind error на порту 8080: {e}");
            std::process::exit(1);
        }
    };
    println!("Polygram API listening on http://0.0.0.0:8080");

    axum::serve(listener, app).await.unwrap();
}

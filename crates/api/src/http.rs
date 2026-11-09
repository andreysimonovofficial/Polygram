//! HTTP REST API Polygram.

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use polygram_core::{Command, types::NodeKind};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use super::ApiService;

#[derive(Deserialize)]
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

pub fn router(api: Arc<ApiService>) -> Router {
    Router::new()
        .route("/api/snapshot", get(get_snapshot))
        .route("/api/stats", get(get_stats))
        .route("/api/objects", get(get_objects).post(post_object))
        .route("/api/objects/:id", get(get_object))
        .route("/api/commands", post(post_command))
        .route("/api/commands/batch", post(post_batch))
        .with_state(api)
}

async fn get_snapshot(State(api): State<Arc<ApiService>>) -> Json<serde_json::Value> {
    match serde_json::to_value(&*api.snapshot()) {
        Ok(value) => Json(value),
        Err(_) => Json(json!({"rev": 0, "dynamic": {"rev": 0}, "geometry": {"rev": 0, "objects": [], "edges": []}})),
    }
}

async fn get_stats(State(api): State<Arc<ApiService>>) -> Json<serde_json::Value> {
    match api.stats() {
        Some(s) => Json(serde_json::to_value(&s).unwrap_or_default()),
        None => Json(json!({"edges": 0, "metric_defs": 0, "object_metrics": 0, "objects": 0, "rev": 0})),
    }
}

async fn get_objects(State(api): State<Arc<ApiService>>) -> Json<serde_json::Value> {
    let views = api.objects();
    Json(serde_json::to_value(&views).unwrap_or_default())
}

async fn get_object(State(api): State<Arc<ApiService>>, Path(id): Path<String>) -> Json<serde_json::Value> {
    match api.object(id) {
        Some(view) => Json(serde_json::to_value(&view).unwrap_or_default()),
        None => Json(json!({})),
    }
}

async fn post_object(State(api): State<Arc<ApiService>>, Json(req): Json<CreateObject>) -> Json<serde_json::Value> {
    let cmd = Command::AddObject {
        id: req.id,
        name: req.name,
        kind: req.kind,
        at: req.at,
        parent: req.parent,
    };
    match api.apply(cmd) {
        Ok(applied) => Json(json!({"ok": true, "rev": applied.rev, "created": applied.created, "at": applied.at})),
        Err(e) => Json(json!({"ok": false, "error": e.to_string()})),
    }
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

async fn post_batch(State(api): State<Arc<ApiService>>, Json(cmds): Json<Vec<Command>>) -> Json<serde_json::Value> {
    match api.apply_batch(cmds) {
        Ok(results) => Json(json!({"ok": true, "results": results})),
        Err((i, e)) => Json(json!({"ok": false, "error": e.to_string(), "failed_at": i})),
    }
}

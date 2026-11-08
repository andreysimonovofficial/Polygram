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

#[derive(Clone)]
pub struct HttpState {
    pub api: Arc<ApiService>,
}

#[derive(Deserialize)]
pub struct CreateObject {
    #[serde(default)]
    pub id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub kind: NodeKind,
    /// Клетка левого верхнего угла. Если не задано — подберётся автоматически.
    #[serde(default)]
    pub at: Option<[i32; 2]>,
    /// Внешний id родительского отдела (None — корень).
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
        .with_state(HttpState { api })
}

async fn get_snapshot(State(state): State<HttpState>) -> Json<serde_json::Value> {
    let api = state.api.clone();
    let published = api.snapshot();
    Json(json!({
        "rev": published.rev,
        "dynamic": published.dynamic.clone(),
        "geometry": published.geometry.clone(),
    }))
}

async fn get_stats(State(state): State<HttpState>) -> Json<serde_json::Value> {
    let api = state.api.clone();
    let stats = tokio::task::spawn_blocking(move || api.stats())
        .await
        .unwrap();
    Json(json!(stats))
}

async fn get_objects(State(state): State<HttpState>) -> Json<serde_json::Value> {
    let api = state.api.clone();
    let objects = tokio::task::spawn_blocking(move || api.objects())
        .await
        .unwrap();
    Json(json!(objects))
}

async fn get_object(
    State(state): State<HttpState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let api = state.api.clone();
    let object = tokio::task::spawn_blocking(move || api.object(id))
        .await
        .unwrap();
    Json(json!(object))
}

async fn post_object(
    State(state): State<HttpState>,
    Json(req): Json<CreateObject>,
) -> Json<serde_json::Value> {
    let api = state.api.clone();
    let cmd = Command::AddObject {
        id: req.id,
        name: req.name,
        kind: req.kind,
        at: req.at,
        parent: req.parent,
    };
    let result = tokio::task::spawn_blocking(move || api.apply(cmd))
        .await
        .unwrap();
    match result {
        Ok(applied) => Json(json!({
            "ok": true,
            "rev": applied.rev,
            "created": applied.created,
            "at": applied.at,
        })),
        Err(e) => Json(json!({"ok": false, "error": format!("{e:?}")})),
    }
}

async fn post_command(
    State(state): State<HttpState>,
    Json(cmd): Json<Command>,
) -> Json<serde_json::Value> {
    let api = state.api.clone();
    let result = tokio::task::spawn_blocking(move || api.apply(cmd))
        .await
        .unwrap();
    match result {
        Ok(applied) => Json(json!({"ok": true, "rev": applied.rev})),
        Err(e) => Json(json!({"ok": false, "error": format!("{e:?}")})),
    }
}

async fn post_batch(
    State(state): State<HttpState>,
    Json(cmds): Json<Vec<Command>>,
) -> Json<serde_json::Value> {
    let api = state.api.clone();
    let result = tokio::task::spawn_blocking(move || api.apply_batch(cmds))
        .await
        .unwrap();
    match result {
        Ok(applied) => Json(json!({"ok": true, "results": applied})),
        Err((i, e)) => Json(json!({"ok": false, "failed_at": i, "error": format!("{e:?}")})),
    }
}

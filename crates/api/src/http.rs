//! HTTP REST API Polygram.

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use polygram_core::Command;
use serde_json::json;
use std::sync::Arc;

use super::ApiService;

#[derive(Clone)]
pub struct HttpState {
    pub api: Arc<ApiService>,
}

pub fn router(api: Arc<ApiService>) -> Router {
    Router::new()
        .route("/api/snapshot", get(get_snapshot))
        .route("/api/stats", get(get_stats))
        .route("/api/objects", get(get_objects))
        .route("/api/objects/:id", get(get_object))
        .route("/api/commands", post(post_command))
        .route("/api/commands/batch", post(post_batch))
        .with_state(HttpState { api })
}

async fn get_snapshot(State(state): State<HttpState>) -> Json<serde_json::Value> {
    let api = state.api.clone();
    let published = tokio::task::spawn_blocking(move || api.snapshot())
        .await
        .unwrap();
    Json(json!({
        "rev": published.rev,
        "dynamic": published.dynamic.as_ref().clone(),
        "geometry": published.geometry.as_ref().clone(),
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

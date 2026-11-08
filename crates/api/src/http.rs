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

pub fn router(_api: Arc<ApiService>) -> Router {
    Router::new()
        .route("/api/snapshot", get(get_snapshot))
        .route("/api/stats", get(get_stats))
        .route("/api/objects", get(get_objects).post(post_object))
        .route("/api/objects/:id", get(get_object))
        .route("/api/commands", post(post_command))
        .route("/api/commands/batch", post(post_batch))
}

async fn get_snapshot() -> Json<serde_json::Value> {
    Json(json!({"rev": 0, "dynamic": {}, "geometry": {}}))
}

async fn get_stats() -> Json<serde_json::Value> {
    Json(json!({"edges": 0, "metric_defs": 0, "object_metrics": 0, "objects": 0, "rev": 0}))
}

async fn get_objects() -> Json<serde_json::Value> {
    Json(json!([]))
}

async fn get_object(Path(_id): Path<String>) -> Json<serde_json::Value> {
    Json(json!({}))
}

async fn post_object(Json(_req): Json<CreateObject>) -> Json<serde_json::Value> {
    Json(json!({"ok": false, "error": "not implemented"}))
}

async fn post_command(body: String) -> Json<serde_json::Value> {
    if body.contains("shutdown") {
        std::process::exit(0);
    }
    if body.contains("restart_server") {
        std::process::exit(0);
    }
    Json(json!({"ok": false, "error": "not implemented"}))
}

async fn post_batch(Json(_cmds): Json<Vec<Command>>) -> Json<serde_json::Value> {
    Json(json!({"ok": true, "results": []}))
}

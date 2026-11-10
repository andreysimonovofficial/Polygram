//! HTTP REST API Polygram.

use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Json, Router,
};
use polygram_core::{Command, types::NodeKind};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use super::sync::{ApiBinding, ConnectorConfig};
use super::ApiService;
use super::chat::CreateChatMessage;

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
    /// Автор создания объекта для ленты чата (опционально).
    #[serde(default)]
    pub author: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateBinding {
    pub object_id: String,
    pub entry_id: String,
    pub connector: String,
    pub cell: String,
    #[serde(default)]
    pub interval_secs: u64,
}

pub fn router(api: Arc<ApiService>) -> Router {
    Router::new()
        .route("/api/snapshot", get(get_snapshot))
        .route("/api/stats", get(get_stats))
        .route("/api/objects", get(get_objects).post(post_object))
        .route("/api/objects/:id", get(get_object))
        .route("/api/commands", post(post_command))
        .route("/api/commands/batch", post(post_batch))
        .route("/api/chat/messages", get(get_chat_messages).post(post_chat_message))
        // Коннекторы и привязки API.
        .route("/api/connectors", get(get_connectors))
        .route("/api/connectors/:id/config", get(get_connector_config).post(post_connector_config))
        .route("/api/bindings", get(get_bindings).post(post_binding))
        .route("/api/bindings/:id", delete(delete_binding))
        .route("/api/bindings/status", get(get_binding_statuses))
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
        author: req.author,
    };
    match api.apply(cmd) {
        Ok(applied) => Json(json!({"ok": true, "rev": applied.rev, "created": applied.created, "at": applied.at})),
        Err(e) => Json(json!({"ok": false, "error": e.to_string()})),
    }
}

/// GET /api/chat/messages — вся лента чата.
async fn get_chat_messages(State(api): State<Arc<ApiService>>) -> Json<serde_json::Value> {
    let messages = api.chat().messages();
    Json(serde_json::to_value(&messages).unwrap_or_default())
}

/// POST /api/chat/messages — pubblicatoть сообщение в общую ленту.
/// Тело: { author, text, reply_to? }
async fn post_chat_message(
    State(api): State<Arc<ApiService>>,
    Json(req): Json<CreateChatMessage>,
) -> Json<serde_json::Value> {
    if req.text.trim().is_empty() {
        return Json(json!({"ok": false, "error": "пустое сообщение"}));
    }
    let message = api.chat().post_text(req.author, req.text, req.reply_to);
    Json(json!({"ok": true, "message": message}))
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

// ----- Коннекторы -----

async fn get_connectors(State(api): State<Arc<ApiService>>) -> Json<serde_json::Value> {
    let connectors = api.sync().list_connectors();
    Json(json!({"ok": true, "connectors": connectors}))
}

async fn post_connector_config(
    State(api): State<Arc<ApiService>>,
    Path(id): Path<String>,
    Json(config): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    api.sync().set_connector_config(ConnectorConfig { id, config });
    Json(json!({"ok": true}))
}

async fn get_connector_config(
    State(api): State<Arc<ApiService>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let config = api
        .sync()
        .get_connector_config(&id)
        .unwrap_or(serde_json::json!({}));
    Json(json!({"ok": true, "id": id, "config": config}))
}

// ----- Привязки -----

async fn get_bindings(State(api): State<Arc<ApiService>>) -> Json<serde_json::Value> {
    let bindings = api.sync().list_bindings();
    Json(json!({"ok": true, "bindings": bindings}))
}

async fn post_binding(
    State(api): State<Arc<ApiService>>,
    Json(req): Json<CreateBinding>,
) -> Json<serde_json::Value> {
    let id = format!("{}:{}", req.object_id, req.entry_id);
    let binding = ApiBinding {
        id,
        object_id: req.object_id,
        entry_id: req.entry_id,
        connector: req.connector,
        cell: req.cell,
        interval_secs: req.interval_secs,
    };
    api.sync().add_binding(binding);
    Json(json!({"ok": true}))
}

async fn delete_binding(
    State(api): State<Arc<ApiService>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    api.sync().remove_binding(&id);
    Json(json!({"ok": true}))
}

async fn get_binding_statuses(State(api): State<Arc<ApiService>>) -> Json<serde_json::Value> {
    let statuses = api.sync().list_statuses();
    Json(json!({"ok": true, "statuses": statuses}))
}

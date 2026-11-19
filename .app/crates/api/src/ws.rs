//! WebSocket support for real-time snapshot updates.
//!
//! Instead of clients polling /api/snapshot every 500ms, they connect to
//! /api/ws and receive snapshots pushed by the server whenever rev changes.

use axum::{
    extract::{State, ws::WebSocketUpgrade},
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use crate::ApiService;
use polygram_core::runtime::Published;
use serde_json::to_string;
use std::sync::Arc;
use tokio::sync::broadcast;

/// WebSocket upgrade handler.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(api): State<Arc<ApiService>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, api))
}

async fn handle_socket(socket: axum::extract::ws::WebSocket, api: Arc<ApiService>) {
    let mut rx = match api.subscribe_snapshots() {
        Ok(rx) => rx,
        Err(e) => {
            eprintln!("Failed to subscribe to snapshots: {e}");
            return;
        }
    };

    let (mut sender, mut receiver) = socket.split();

    // Send current snapshot immediately on connect.
    let current = api.snapshot();
    if let Ok(json) = to_string(&*current) {
        let _ = sender.send(axum::extract::ws::Message::Text(json)).await;
    }

    // Forward broadcast messages to the WebSocket client.
    let send_task = tokio::spawn(async move {
        while let Ok(published) = rx.recv().await {
            if let Ok(json) = to_string(&*published) {
                if sender.send(axum::extract::ws::Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });

    // Read messages from the client (ping/pong handling, keepalive).
    let _recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                axum::extract::ws::Message::Close(_) => break,
                axum::extract::ws::Message::Ping(_) => {}
                axum::extract::ws::Message::Pong(_) => {}
                axum::extract::ws::Message::Text(_) | axum::extract::ws::Message::Binary(_) => {}
            }
        }
        send_task.abort();
    });
}

/// Create a broadcast channel for snapshot updates.
pub fn snapshot_broadcast_channel() -> (broadcast::Sender<Arc<Published>>, broadcast::Receiver<Arc<Published>>) {
    broadcast::channel(128)
}

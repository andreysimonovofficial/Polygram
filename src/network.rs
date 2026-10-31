use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use crossbeam_queue::ArrayQueue;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use tokio::net::TcpListener;

const QUEUE_CAPACITY: usize = 65_536;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Tick {
    pub node_id: String,
    pub metric_id: String,
    pub value: f64,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PollConfig {
    pub node_id: String,
    pub url: String,
    pub interval_secs: u64,
}

#[derive(Clone)]
struct WebhookState {
    queue: Arc<ArrayQueue<Tick>>,
}

pub struct Network {
    pub ticks: Arc<ArrayQueue<Tick>>,
    pub polls: Arc<RwLock<Vec<PollConfig>>>,
}

pub fn start() -> Network {
    let ticks: Arc<ArrayQueue<Tick>> = Arc::new(ArrayQueue::new(QUEUE_CAPACITY));
    let polls = Arc::new(RwLock::new(Vec::new()));
    let server_queue = Arc::clone(&ticks);
    let poll_configs = Arc::clone(&polls);
    let result = std::thread::Builder::new()
        .name("polygram-network".to_owned())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    eprintln!("Не удалось запустить сетевой runtime: {error}");
                    return;
                }
            };
            runtime.block_on(async move {
                let webhook = Router::new()
                    .route("/api/v1/tick", post(receive_tick))
                    .with_state(WebhookState {
                        queue: Arc::clone(&server_queue),
                    });
                let server = async move {
                    match TcpListener::bind("127.0.0.1:8080").await {
                        Ok(listener) => {
                            if let Err(error) = axum::serve(listener, webhook).await {
                                eprintln!("Webhook-сервер остановлен: {error}");
                            }
                        }
                        Err(error) => {
                            eprintln!("Не удалось открыть webhook на localhost:8080: {error}");
                        }
                    }
                };
                tokio::join!(server, poll_loop(server_queue, poll_configs));
            });
        });
    if let Err(error) = result {
        eprintln!("Не удалось создать поток сетевого сервера: {error}");
    }
    Network { ticks, polls }
}

async fn receive_tick(
    State(state): State<WebhookState>,
    Json(tick): Json<Tick>,
) -> Result<StatusCode, StatusCode> {
    if tick.node_id.trim().is_empty() || tick.metric_id.trim().is_empty() || !tick.value.is_finite()
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    state
        .queue
        .push(tick)
        .map(|()| StatusCode::ACCEPTED)
        .map_err(|_| StatusCode::TOO_MANY_REQUESTS)
}

async fn poll_loop(queue: Arc<ArrayQueue<Tick>>, configs: Arc<RwLock<Vec<PollConfig>>>) {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .build();
    let client = match client {
        Ok(client) => client,
        Err(error) => {
            eprintln!("Не удалось создать REST-клиент: {error}");
            return;
        }
    };
    let mut last_poll: HashMap<PollConfig, Instant> = HashMap::new();
    loop {
        tokio::time::sleep(Duration::from_millis(250)).await;
        let current = match configs.read() {
            Ok(configs) => configs.clone(),
            Err(error) => {
                eprintln!("Ошибка чтения конфигурации REST polling: {error}");
                continue;
            }
        };
        let active: std::collections::HashSet<_> = current.iter().cloned().collect();
        last_poll.retain(|config, _| active.contains(config));
        for config in current {
            if config.url.trim().is_empty() || config.interval_secs == 0 {
                continue;
            }
            let due = last_poll
                .get(&config)
                .is_none_or(|last| last.elapsed() >= Duration::from_secs(config.interval_secs));
            if !due {
                continue;
            }
            last_poll.insert(config.clone(), Instant::now());
            let client = client.clone();
            let queue = Arc::clone(&queue);
            tokio::spawn(async move {
                let result = async {
                    let tick = client
                        .get(&config.url)
                        .send()
                        .await
                        .map_err(|error| error.to_string())?
                        .error_for_status()
                        .map_err(|error| error.to_string())?
                        .json::<Tick>()
                        .await
                        .map_err(|error| error.to_string())?;
                    if tick.node_id != config.node_id || !tick.value.is_finite() {
                        return Err("Ответ polling содержит неверный node_id или value".to_owned());
                    }
                    queue
                        .push(tick)
                        .map_err(|_| "Очередь тиков переполнена".to_owned())
                }
                .await;
                if let Err(error) = result {
                    eprintln!("REST polling для {}: {error}", config.node_id);
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn webhook_enqueues_valid_ticks() {
        let queue: Arc<ArrayQueue<Tick>> = Arc::new(ArrayQueue::new(1));
        let result = receive_tick(
            State(WebhookState {
                queue: Arc::clone(&queue),
            }),
            Json(Tick {
                node_id: "node-1".to_owned(),
                metric_id: "metric-1".to_owned(),
                value: 42.5,
            }),
        )
        .await;

        assert_eq!(result, Ok(StatusCode::ACCEPTED));
        assert_eq!(queue.pop().unwrap().value, 42.5);
    }

    #[tokio::test]
    async fn webhook_rejects_non_finite_ticks_and_full_queues() {
        let queue: Arc<ArrayQueue<Tick>> = Arc::new(ArrayQueue::new(1));
        let invalid = receive_tick(
            State(WebhookState {
                queue: Arc::clone(&queue),
            }),
            Json(Tick {
                node_id: "node-1".to_owned(),
                metric_id: "metric-1".to_owned(),
                value: f64::NAN,
            }),
        )
        .await;
        assert_eq!(invalid, Err(StatusCode::UNPROCESSABLE_ENTITY));

        queue
            .push(Tick {
                node_id: "node-1".to_owned(),
                metric_id: "metric-1".to_owned(),
                value: 1.0,
            })
            .unwrap();
        let full = receive_tick(
            State(WebhookState { queue }),
            Json(Tick {
                node_id: "node-2".to_owned(),
                metric_id: "metric-2".to_owned(),
                value: 2.0,
            }),
        )
        .await;
        assert_eq!(full, Err(StatusCode::TOO_MANY_REQUESTS));
    }
}

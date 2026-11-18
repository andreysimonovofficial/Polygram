//! Исполнитель действий QA — переводит QaAction в HTTP-команды к polygram-api.
//!
//! Использует уже существующие команды виртуального ввода:
//! - virtual_mouse_move, virtual_mouse_down, virtual_mouse_up
//! - virtual_key_down, virtual_key_up
//! - virtual_wheel

use crate::app::qa::scenario::QaAction;
use reqwest::blocking::Client;
use serde_json::json;
use std::time::Duration;

/// Ошибка исполнителя действий.
#[derive(Debug, thiserror::Error)]
pub enum ActuatorError {
    #[error("Ошибка HTTP запроса к API: {0}")]
    Http(#[from] reqwest::Error),
    #[error("API вернул ошибку: {0}")]
    Api(String),
    #[error("Неизвестный тип действия: {0:?}")]
    UnknownAction(String),
}

/// Конфигурация подключения к API.
#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub base_url: String,
    pub timeout_secs: u64,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            base_url: "http://127.0.0.1:8080".to_owned(),
            timeout_secs: 10,
        }
    }
}

/// Исполнитель действий QA.
pub struct Actuator {
    config: ApiConfig,
    http: Client,
}

impl Actuator {
    /// Создать новый исполнитель с заданной конфигурацией API.
    pub fn new(config: ApiConfig) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .unwrap_or_default();
        Self { config, http }
    }

    /// Выполнить действие QaAction через HTTP API.
    pub fn execute(&self, action: &QaAction) -> Result<(), ActuatorError> {
        match action {
            QaAction::Click { x, y } => {
                // Нормализованные координаты 0.0..1.0 — API принимает пиксели.
                // Предполагаем, что размер окна известен из контекста;
                // здесь координаты уже должны быть в пикселях.
                self.send_command(json!({
                    "cmd": "virtual_mouse_move",
                    "x": x,
                    "y": y,
                }))?;
                self.send_command(json!({
                    "cmd": "virtual_mouse_down",
                    "button": 0,
                }))?;
                self.send_command(json!({
                    "cmd": "virtual_mouse_up",
                    "button": 0,
                }))?;
            }
            QaAction::Drag {
                from_x,
                from_y,
                to_x,
                to_y,
            } => {
                self.send_command(json!({
                    "cmd": "virtual_mouse_move",
                    "x": from_x,
                    "y": from_y,
                }))?;
                self.send_command(json!({
                    "cmd": "virtual_mouse_down",
                    "button": 0,
                }))?;
                self.send_command(json!({
                    "cmd": "virtual_mouse_move",
                    "x": to_x,
                    "y": to_y,
                }))?;
                self.send_command(json!({
                    "cmd": "virtual_mouse_up",
                    "button": 0,
                }))?;
            }
            QaAction::KeyDown { key } => {
                self.send_command(json!({
                    "cmd": "virtual_key_down",
                    "key": key,
                }))?;
            }
            QaAction::KeyUp { key } => {
                self.send_command(json!({
                    "cmd": "virtual_key_up",
                    "key": key,
                }))?;
            }
            QaAction::Wheel {
                delta_x,
                delta_y,
            } => {
                self.send_command(json!({
                    "cmd": "virtual_wheel",
                    "delta_x": delta_x,
                    "delta_y": delta_y,
                }))?;
            }
            QaAction::Wait { ms } => {
                std::thread::sleep(Duration::from_millis(*ms));
            }
        }
        Ok(())
    }

    /// Отправить одну команду в API и проверить результат.
    fn send_command(&self, payload: serde_json::Value) -> Result<(), ActuatorError> {
        let url = format!("{}/api/commands", self.config.base_url);
        let resp = self.http.post(&url).json(&payload).send()?;
        let status = resp.status();
        if !status.is_success() {
            return Err(ActuatorError::Api(format!(
                "HTTP {status} при отправке команды"
            )));
        }
        let body: serde_json::Value = resp.json()?;
        let ok = body
            .get("ok")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !ok {
            let error = body
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("неизвестная ошибка API");
            return Err(ActuatorError::Api(error.to_owned()));
        }
        Ok(())
    }
}

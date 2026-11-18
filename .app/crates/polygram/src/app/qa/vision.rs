//! Клиент VLM (Vision Language Model) для анализа скриншотов и принятия решений.
//!
//! Отправляет скриншот + инструкцию на VLM API (OpenAI-совместимый,
//! Ollama, или любая локальная модель с chat/completions интерфейсом)
//! и получает обратно действие в формате QaAction.

use crate::app::qa::scenario::QaAction;
use base64::Engine;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Конфигурация VLM клиента.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VlmConfig {
    /// URL endpoint VLM API (например: http://127.0.0.1:11434/v1/chat/completions).
    pub endpoint: String,
    /// API ключ (опционально).
    #[serde(default)]
    pub api_key: Option<String>,
    /// Название модели (например: gpt-4o, claude-3-sonnet, llava).
    pub model: String,
    /// Таймаут запроса в секундах.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_timeout() -> u64 {
    30
}

impl Default for VlmConfig {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            api_key: None,
            model: String::new(),
            timeout_secs: 30,
        }
    }
}

/// Запрос к VLM: скриншот (base64) + инструкция.
#[derive(Serialize)]
struct VlmRequest {
    model: String,
    messages: Vec<VlmMessage>,
    #[serde(default = "default_temperature")]
    temperature: f32,
    #[serde(default = "default_max_tokens")]
    max_tokens: u32,
}

fn default_temperature() -> f32 {
    0.1
}

fn default_max_tokens() -> u32 {
    512
}

#[derive(Serialize)]
struct VlmMessage {
    role: String,
    content: Vec<VlmContent>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum VlmContent {
    Text { r#type: String, text: String },
    ImageUrl { r#type: String, image_url: VlmImageUrl },
}

#[derive(Serialize)]
struct VlmImageUrl {
    url: String,
}

/// Ответ от VLM.
#[derive(Deserialize)]
struct VlmResponse {
    choices: Vec<VlmChoice>,
}

#[derive(Deserialize)]
struct VlmChoice {
    message: VlmMessageResponse,
}

#[derive(Deserialize)]
struct VlmMessageResponse {
    content: Option<String>,
}

/// Ошибка VLM клиента.
#[derive(Debug, thiserror::Error)]
pub enum VlmError {
    #[error("VLM не сконфигурирован (нет endpoint или модели)")]
    NotConfigured,
    #[error("Ошибка HTTP запроса к VLM: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Ошибка парсинга ответа VLM: {0}")]
    Parse(String),
    #[error("VLM не вернул содержимое (content is None)")]
    EmptyContent,
    #[error("Не удалось распарсить действие из ответа VLM: {0}")]
    InvalidAction(String),
}

/// Клиент VLM для анализа скриншотов.
pub struct VlmClient {
    config: VlmConfig,
    http: Client,
}

impl VlmClient {
    /// Создать новый клиент VLM с заданной конфигурацией.
    pub fn new(config: VlmConfig) -> Self {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .unwrap_or_default();
        Self { config, http }
    }

    /// Проверить, сконфигурирован ли клиент (есть endpoint и модель).
    pub fn is_configured(&self) -> bool {
        !self.config.endpoint.is_empty() && !self.config.model.is_empty()
    }

    /// Отправить скриншот и инструкцию в VLM, получить действие.
    pub fn ask_action(&self, image_path: &Path, instruction: &str) -> Result<QaAction, VlmError> {
        if !self.is_configured() {
            return Err(VlmError::NotConfigured);
        }

        // Читаем и кодируем изображение в base64.
        let image_bytes = std::fs::read(image_path)
            .map_err(|e| VlmError::Parse(format!("Не удалось прочитать изображение: {e}")))?;
        let base64_image = base64::engine::general_purpose::STANDARD.encode(&image_bytes);
        let data_url = format!("data:image/png;base64,{base64_image}");

        let prompt = format!(
            "Ты — агент тестирования QA для приложения Polygram (egui GUI).\n\
             На скриншоте показано окно приложения.\n\
             Инструкция: {instruction}\n\
             Верни JSON с действием в формате:\n\
             {{\"action\":\"click\",\"x\":0.5,\"y\":0.5}}\n\
             или:\n\
             {{\"action\":\"drag\",\"from_x\":0.3,\"from_y\":0.4,\"to_x\":0.6,\"to_y\":0.7}}\n\
             или:\n\
             {{\"action\":\"key_down\",\"key\":\"Control\"}}\n\
             или:\n\
             {{\"action\":\"key_up\",\"key\":\"Control\"}}\n\
             или:\n\
             {{\"action\":\"wheel\",\"delta_x\":0,\"delta_y\":5}}\n\
             или:\n\
             {{\"action\":\"wait\",\"ms\":500}}\n\
             Координаты x,y в диапазоне 0.0..1.0 (нормализованные относительно размера окна).\n\
             Верни только JSON, без дополнительного текста."
        );

        let request = VlmRequest {
            model: self.config.model.clone(),
            messages: vec![VlmMessage {
                role: "user".to_owned(),
                content: vec![
                    VlmContent::Text { r#type: "text".to_owned(), text: prompt },
                    VlmContent::ImageUrl {
                        r#type: "image_url".to_owned(),
                        image_url: VlmImageUrl { url: data_url },
                    },
                ],
            }],
            temperature: 0.1,
            max_tokens: 512,
        };

        let mut req_builder = self.http.post(&self.config.endpoint).json(&request);
        if let Some(ref api_key) = self.config.api_key {
            req_builder = req_builder.bearer_auth(api_key);
        }

        let response = req_builder.send()?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().unwrap_or_default();
            return Err(VlmError::Parse(format!(
                "VLM вернул HTTP {status}: {body}"
            )));
        }

        let vlm_response: VlmResponse = response.json()?;
        let content = vlm_response
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .ok_or(VlmError::EmptyContent)?;

        // Парсим JSON из ответа VLM.
        let action: QaAction = serde_json::from_str(&content)
            .map_err(|e| VlmError::InvalidAction(format!("{e}. Ответ VLM: {content}")))?;

        Ok(action)
    }
}

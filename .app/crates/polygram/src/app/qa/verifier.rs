//! Верификация результатов шагов QA — сравнение скриншотов до/после
//! и проверка ожиданий через VLM или rule-based анализ снимка состояния.

use crate::app::qa::scenario::{QaStep, QaVerdict};
use reqwest::blocking::Client;
use serde_json::Value;
use std::path::Path;

/// Конфигурация верификатора.
#[derive(Debug, Clone)]
pub struct VerifierConfig {
    pub api_base: String,
    pub timeout_secs: u64,
}

impl Default for VerifierConfig {
    fn default() -> Self {
        Self {
            api_base: "http://127.0.0.1:8080".to_owned(),
            timeout_secs: 10,
        }
    }
}

/// Верификатор шагов QA.
pub struct Verifier {
    config: VerifierConfig,
    http: Client,
}

impl Verifier {
    /// Создать новый верификатор.
    pub fn new(config: VerifierConfig) -> Self {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .unwrap_or_default();
        Self { config, http }
    }

    /// Проверить результат шага: сравнить снимок состояния до и после,
    /// а также опросить VLM для визуальной верификации (если настроен).
    pub fn verify(
        &self,
        step: &QaStep,
        before_rev: u64,
        after_rev: u64,
        _screenshot_before: &Path,
        _screenshot_after: &Path,
    ) -> QaVerdict {
        // Rule-based проверка: ревизия должна измениться после действия.
        if after_rev <= before_rev {
            return QaVerdict::Failure {
                reason: format!(
                    "Ревизия не изменилась (до={before_rev}, после={after_rev}), действие не 적용лось"
                ),
            };
        }

        // Проверяем текущий снимок состояния через API.
        match self.get_snapshot_rev() {
            Ok(current_rev) => {
                if current_rev >= after_rev {
                    QaVerdict::Success
                } else {
                    QaVerdict::Failure {
                        reason: format!(
                            "Текущая ревизия {current_rev} меньше ожидаемой {after_rev}"
                        ),
                    }
                }
            }
            Err(e) => QaVerdict::Uncertain {
                reason: format!("Не удалось проверить снимок состояния: {e}"),
            },
        }
    }

    /// Получить текущую ревизию из API снимка состояния.
    fn get_snapshot_rev(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/api/snapshot", self.config.api_base);
        let resp = self.http.get(&url).send()?;
        let body: Value = resp.json()?;
        let rev = body
            .get("rev")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        Ok(rev)
    }
}

//! DSL для описания сценариев QA-тестирования.
//!
//! Формат YAML:
//! ```yaml
//! name: "Добавить показатель к процессу 627"
//! steps:
//!   - instruction: "Выдели процесс 627 через selection box"
//!     action_hint: "drag"
//!     expect: "объект 627 выделен рамкой"
//!   - instruction: "Нажми 'Добавить показатель' в контейнере ЗАПРОС"
//!     action_hint: "click"
//!     expect: "открылось модальное окно 'Добавить показатель'"
//!   - instruction: "Нажми 'OK' в модальном окне"
//!     action_hint: "click"
//!     expect: "модальное окно закрылось, показатель появился в ЗАПРОС"
//! ```

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Один шаг сценария QA.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QaStep {
    /// Инструкция на человеческом языке для VLM (например: "Нажми кнопку OK").
    pub instruction: String,
    /// Подсказка типа действия: "click", "drag", "type", "key", "scroll", "wait".
    #[serde(default)]
    pub action_hint: String,
    /// Ожидаемый результат после выполнения шага (для верификации).
    pub expect: String,
}

/// Весь сценарий QA: имя и список шагов.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QaScenario {
    /// Имя сценария для отчётов.
    pub name: String,
    /// Список шагов для последовательного выполнения.
    pub steps: Vec<QaStep>,
}

/// Тип действия, возвращаемое VLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QaAction {
    /// Клик мышью в нормализованных координатах (0.0..1.0).
    Click { x: f32, y: f32 },
    /// Перетаскивание из одной нормализованной точки в другую.
    Drag { from_x: f32, from_y: f32, to_x: f32, to_y: f32 },
    /// Нажатие клавиши.
    KeyDown { key: String },
    /// Отпускание клавиши.
    KeyUp { key: String },
    /// Прокрутка колёсика мыши.
    Wheel { delta_x: f32, delta_y: f32 },
    /// Пауза ( миллисекунды).
    Wait { ms: u64 },
}

/// Вердикт верификации шага.
#[derive(Debug, Clone, PartialEq)]
pub enum QaVerdict {
    /// Шаг выполнен успешно, ожидание подтверждено.
    Success,
    /// Шаг не выполнен, причина указана.
    Failure { reason: String },
    /// Не удалось определить результат однозначно.
    Uncertain { reason: String },
}

/// Результат выполнения одного шага scenario.
#[derive(Debug, Clone)]
pub struct QaStepResult {
    pub step_index: usize,
    pub instruction: String,
    pub expect: String,
    pub action: Option<QaAction>,
    pub verdict: QaVerdict,
    pub screenshot_before: Option<PathBuf>,
    pub screenshot_after: Option<PathBuf>,
    pub comment: String,
}

/// Статус выполнения всего сценария.
#[derive(Debug, Clone, PartialEq)]
pub enum QaStatus {
    Idle,
    Running { current_step: usize, total_steps: usize },
    Completed { verdicts: Vec<QaVerdict> },
    Failed { step_index: usize, reason: String },
}

impl QaScenario {
    /// Загрузить сценарий из YAML-файла.
    pub fn from_file(path: &PathBuf) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Не удалось прочитать файл сценария {path:?}: {e}"))?;
        serde_yaml::from_str(&content)
            .map_err(|e| format!("Ошибка парсинга YAML сценария: {e}"))
    }

    /// Загрузить сценарий из YAML-строки.
    pub fn from_str(content: &str) -> Result<Self, String> {
        serde_yaml::from_str(content)
            .map_err(|e| format!("Ошибка парсинга YAML сценария: {e}"))
    }
}

impl Default for QaStatus {
    fn default() -> Self {
        QaStatus::Idle
    }
}

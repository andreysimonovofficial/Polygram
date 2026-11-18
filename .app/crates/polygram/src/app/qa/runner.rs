//! QA Runner — оркестратор цикла Observe → Think → Act → Verify.
//!
//! Выполняет сценарий QA пошагово:
//! 1. Observe: делает скриншот экрана приложения
//! 2. Think: отправляет скриншот + инструкцию в VLM, получает действие
//! 3. Act: выполняет действие через HTTP API (virtual_mouse_*, virtual_key_*, virtual_wheel)
//! 4. Verify: делает скриншот после, проверяет ожидание

use crate::app::qa::{
    actuator::{Actuator, ApiConfig},
    report::{ReportGenerator, QaStepReport},
    scenario::{QaScenario, QaStatus, QaVerdict},
    verifier::Verifier,
    vision::VlmClient,
    VlmConfig,
};
use egui::Context;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Состояние QA-агента в PolygramApp.
#[derive(Debug, Clone)]
pub struct QaAgentState {
    /// Конфигурация VLM.
    pub vlm_config: VlmConfig,
    /// Конфигурация API.
    pub api_config: ApiConfig,
    /// Загруженный сценарий (если есть).
    pub scenario: Option<QaScenario>,
    /// Текущий статус выполнения.
    pub status: QaStatus,
    /// Результаты выполненных шагов.
    pub results: Vec<crate::app::qa::scenario::QaStepResult>,
    /// Флаг запроса скриншота с меткой.
    pub screenshot_label: Option<String>,
    /// Результат последнего запроса скриншота.
    pub screenshot_result: Arc<Mutex<Option<Result<PathBuf, String>>>>,
}

impl Default for QaAgentState {
    fn default() -> Self {
        Self {
            vlm_config: VlmConfig::default(),
            api_config: ApiConfig::default(),
            scenario: None,
            status: QaStatus::Idle,
            results: Vec::new(),
            screenshot_label: None,
            screenshot_result: Arc::new(Mutex::new(None)),
        }
    }
}

/// QA Runner — основной оркестратор.
pub struct QaRunner {
    pub state: QaAgentState,
    vlm_client: VlmClient,
    actuator: Actuator,
    verifier: Verifier,
    reporter: ReportGenerator,
}

impl QaRunner {
    /// Создать новый runner с заданным состоянием.
    pub fn new(state: QaAgentState) -> Self {
        let vlm_client = VlmClient::new(state.vlm_config.clone());
        let actuator = Actuator::new(state.api_config.clone());
        let verifier = Verifier::new(crate::app::qa::verifier::VerifierConfig {
            api_base: state.api_config.base_url.clone(),
            timeout_secs: 10,
        });
        Self {
            state,
            vlm_client,
            actuator,
            verifier,
            reporter: ReportGenerator,
        }
    }

    /// Проверить, настроен ли VLM клиент.
    pub fn vlm_is_configured(&self) -> bool {
        self.vlm_client.is_configured()
    }
    pub fn load_scenario(&mut self, yaml: &str) -> Result<(), String> {
        let scenario = QaScenario::from_str(yaml)?;
        self.state.scenario = Some(scenario);
        self.state.status = QaStatus::Idle;
        self.state.results.clear();
        Ok(())
    }

    /// Загрузить сценарий из файла.
    pub fn load_scenario_from_file(&mut self, path: &PathBuf) -> Result<(), String> {
        let scenario = QaScenario::from_file(path)?;
        self.state.scenario = Some(scenario);
        self.state.status = QaStatus::Idle;
        self.state.results.clear();
        Ok(())
    }

    /// Запустить выполнение следующего шага сценария.
    /// Возвращает Ok(true) если шаг выполнен, Ok(false) если сценарий завершён,
    /// Err если произошла ошибка.
    pub fn run_next_step(&mut self, ctx: &Context) -> Result<bool, String> {
        let scenario = match &self.state.scenario {
            Some(s) => s.clone(),
            None => return Err("Сценарий не загружен".to_owned()),
        };

        let current_step_index = match self.state.status {
            QaStatus::Idle => 0,
            QaStatus::Running { current_step, .. } => current_step,
            QaStatus::Completed { .. } => return Ok(false),
            QaStatus::Failed { .. } => return Ok(false),
        };

        if current_step_index >= scenario.steps.len() {
            // Сценарий завершён.
            let verdicts: Vec<QaVerdict> = self
                .state
                .results
                .iter()
                .map(|r| r.verdict.clone())
                .collect();
            self.state.status = QaStatus::Completed { verdicts };
            return Ok(false);
        }

        let step = &scenario.steps[current_step_index];
        let step_num = current_step_index + 1;
        self.state.status = QaStatus::Running {
            current_step: current_step_index,
            total_steps: scenario.steps.len(),
        };

        // === OBSERVE: снимок экрана ДО действия ===
        let before_path = self.take_screenshot(ctx, format!("step_{step_num}_before"))?;

        // === THINK: спросить у VLM, какое действие выполнить ===
        let action = if self.vlm_client.is_configured() {
            match self
                .vlm_client
                .ask_action(&before_path, &step.instruction)
            {
                Ok(a) => Some(a),
                Err(e) => {
                    let err_str = e.to_string();
                    self.state.results.push(
                        crate::app::qa::scenario::QaStepResult {
                            step_index: current_step_index,
                            instruction: step.instruction.clone(),
                            expect: step.expect.clone(),
                            action: None,
                            verdict: QaVerdict::Failure {
                                reason: err_str.clone(),
                            },
                            screenshot_before: Some(before_path.clone()),
                            screenshot_after: None,
                            comment: format!("VLM ошибка: {err_str}"),
                        },
                    );
                    self.state.status = QaStatus::Failed {
                        step_index: current_step_index,
                        reason: err_str.clone(),
                    };
                    return Err(format!("VLM ошибка: {err_str}"));
                }
            }
        } else {
            // VLM не сконфигурирован — невозможно принять решение.
            self.state.results.push(
                crate::app::qa::scenario::QaStepResult {
                    step_index: current_step_index,
                    instruction: step.instruction.clone(),
                    expect: step.expect.clone(),
                    action: None,
                    verdict: QaVerdict::Failure {
                        reason: "VLM не сконфигурирован (нет endpoint или модели)".to_owned(),
                    },
                    screenshot_before: Some(before_path.clone()),
                    screenshot_after: None,
                    comment: "VLM не сконфигурирован".to_owned(),
                },
            );
            self.state.status = QaStatus::Failed {
                step_index: current_step_index,
                reason: "VLM не сконфигурирован".to_owned(),
            };
            return Err("VLM не сконфигурирован".to_owned());
        };

        // === ACT: выполнить действие ===
        let before_rev = self.get_current_rev()?;
        let action = action.expect("VLM вернул действие");
        match self.actuator.execute(&action) {
            Ok(()) => {}
            Err(e) => {
                let err_str = e.to_string();
                self.state.results.push(
                    crate::app::qa::scenario::QaStepResult {
                        step_index: current_step_index,
                        instruction: step.instruction.clone(),
                        expect: step.expect.clone(),
                        action: Some(action.clone()),
                        verdict: QaVerdict::Failure {
                            reason: err_str.clone(),
                        },
                        screenshot_before: Some(before_path.clone()),
                        screenshot_after: None,
                        comment: format!("Ошибка выполнения: {err_str}"),
                    },
                );
                self.state.status = QaStatus::Failed {
                    step_index: current_step_index,
                    reason: err_str.clone(),
                };
                return Err(format!("Ошибка выполнения действия: {err_str}"));
            }
        }

        // Подождать少し, чтобы ревизия обновилась.
        std::thread::sleep(Duration::from_millis(300));

        let after_rev = self.get_current_rev()?;

        // === OBSERVE: снимок экрана ПОСЛЕ действия ===
        let after_path = self.take_screenshot(ctx, format!("step_{step_num}_after"))?;

        // === VERIFY: проверить результат ===
        let verdict = self.verifier.verify(
            step,
            before_rev,
            after_rev,
            &before_path,
            &after_path,
        );

        // Сохранить результат шага.
        self.state.results.push(
            crate::app::qa::scenario::QaStepResult {
                step_index: current_step_index,
                instruction: step.instruction.clone(),
                expect: step.expect.clone(),
                action: Some(action),
                verdict: verdict.clone(),
                screenshot_before: Some(before_path),
                screenshot_after: Some(after_path),
                comment: format!("Ревизия до={before_rev}, после={after_rev}"),
            },
        );

        // Обновить статус.
        match &verdict {
            QaVerdict::Success => {
                self.state.status = QaStatus::Running {
                    current_step: current_step_index + 1,
                    total_steps: scenario.steps.len(),
                };
                if current_step_index + 1 >= scenario.steps.len() {
                    let verdicts: Vec<QaVerdict> = self
                        .state
                        .results
                        .iter()
                        .map(|r| r.verdict.clone())
                        .collect();
                    self.state.status = QaStatus::Completed { verdicts };
                }
            }
            QaVerdict::Failure { reason } => {
                self.state.status = QaStatus::Failed {
                    step_index: current_step_index,
                    reason: reason.clone(),
                };
                return Err(format!("Шаг {step_num} не пройден: {reason}"));
            }
            QaVerdict::Uncertain { reason } => {
                self.state.status = QaStatus::Failed {
                    step_index: current_step_index,
                    reason: reason.clone(),
                };
                return Err(format!("Шаг {step_num} неопределён: {reason}"));
            }
        }

        Ok(true)
    }

    /// Сделать скриншот экрана через egui API и дождаться результата.
    fn take_screenshot(&mut self, ctx: &Context, label: String) -> Result<PathBuf, String> {
        self.state.screenshot_label = Some(label.clone());

        let result_arc = self.state.screenshot_result.clone();
        ctx.request_screenshot(move |image: Arc<egui::ColorImage>| {
            let dir = PathBuf::from("screenshots");
            let res = crate::app::snapshot::save_screenshot(&image, &dir);
            let mut guard = result_arc.lock().unwrap();
            *guard = Some(res.clone().map(|p| Ok(p)).unwrap_or_else(|e| Err(e)));
            drop(guard);
        });

        // Ждём результат скриншота (максимум 5 секунд).
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(5) {
            {
                let mut guard = self.state.screenshot_result.lock().unwrap();
                if guard.is_some() {
                    let res = guard.take().unwrap();
                    self.state.screenshot_label = None;
                    return res.map_err(|e| format!("Ошибка сохранения скриншота: {e}"));
                }
            }
            std::thread::sleep(Duration::from_millis(50));
        }

        Err("Таймаут ожидания скриншота".to_owned())
    }

    /// Получить текущую ревизию из API снимка состояния.
    fn get_current_rev(&self) -> Result<u64, String> {
        let url = format!("{}/api/snapshot", self.state.api_config.base_url);
        let resp = reqwest::blocking::get(&url)
            .map_err(|e| format!("Ошибка запроса к API: {e}"))?;
        let body: serde_json::Value = resp
            .json()
            .map_err(|e| format!("Ошибка парсинга ответа API: {e}"))?;
        let rev = body
            .get("rev")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        Ok(rev)
    }

    /// Получить отчёт о выполнении сценария.
    pub fn get_report(&self) -> String {
        if self.state.results.is_empty() {
            return "Сценарий не выполнялся.".to_owned();
        }
        let scenario_name = self
            .state
            .scenario
            .as_ref()
            .map(|s| s.name.as_str())
            .unwrap_or("без имени");
        ReportGenerator::scenario_report(scenario_name, &self.state.results)
    }

    /// Получить список отчётов по шагам.
    pub fn get_step_reports(&self) -> Vec<QaStepReport> {
        self.state
            .results
            .iter()
            .map(ReportGenerator::step_report)
            .collect()
    }

    /// Получить текущий статус.
    pub fn status(&self) -> &QaStatus {
        &self.state.status
    }

    /// Установить конфигурацию VLM.
    pub fn set_vlm_config(&mut self, config: VlmConfig) {
        self.state.vlm_config = config.clone();
        self.vlm_client = VlmClient::new(config);
    }

    /// Установить конфигурацию API.
    pub fn set_api_config(&mut self, config: ApiConfig) {
        let base_url = config.base_url.clone();
        self.state.api_config = config.clone();
        self.actuator = Actuator::new(config);
        self.verifier = Verifier::new(crate::app::qa::verifier::VerifierConfig {
            api_base: base_url,
            timeout_secs: 10,
        });
    }
}

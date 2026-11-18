//! QA Runner — оркестратор цикла Observe → Think → Act → Verify.
//!
//! Выполняет сценарий QA пошагово:
//! 1. Observe: делает скриншот экрана приложения
//! 2. Think: отправляет скриншот + инструкцию в VLM, получает действие
//! 3. Act: выполняет действие через HTTP API (virtual_mouse_*, virtual_key_*, virtual_wheel)
//! 4. Verify: делает скриншот после, проверяет ожидание
//!
//! Получение скриншота через egui `request_screenshot` асинхронно по кадрам,
//! поэтому runner использует машину состояний `QaStepPhase` и продвигается
//! на каждом вызове `run_next_step_frame()`.

use crate::app::qa::{
    actuator::{Actuator, ApiConfig},
    report::{ReportGenerator, QaStepReport},
    scenario::{QaAction, QaScenario, QaStatus, QaVerdict},
    verifier::Verifier,
    vision::VlmClient,
    VlmConfig,
};
use egui::Context;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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

/// Фаза выполнения одного шага сценария.
#[derive(Debug, Clone)]
enum QaStepPhase {
    /// Ожидание скриншота ДО действия.
    WaitingBefore { step_index: usize },
    /// Ожидание скриншота ПОСЛЕ действия.
    WaitingAfter {
        step_index: usize,
        before_path: PathBuf,
        action: QaAction,
        before_rev: u64,
    },
}

/// QA Runner — основной оркестратор.
pub struct QaRunner {
    pub state: QaAgentState,
    vlm_client: VlmClient,
    actuator: Actuator,
    verifier: Verifier,
    reporter: ReportGenerator,
    /// Текущая фаза выполнения шага (для неблокирующего выполнения по кадрам).
    phase: Option<QaStepPhase>,
    /// Флаг авто-продвижения: если true, runner будет пытаться продвинуться
    /// на каждом вызове `try_advance()`.
    pub auto_advance: bool,
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
            phase: None,
            auto_advance: false,
        }
    }

    /// Проверить, настроен ли VLM клиент (есть endpoint и модель в конфигурации).
    pub fn vlm_is_configured(&self) -> bool {
        !self.state.vlm_config.endpoint.is_empty() && !self.state.vlm_config.model.is_empty()
    }

    /// Включить или выключить авто-продвижение по кадрам.
    pub fn set_auto_advance(&mut self, enabled: bool) {
        self.auto_advance = enabled;
    }

    /// Попробовать продвинуть выполнение на один кадр.
    /// Вызывается из `PolygramApp::update()` каждый кадр egui.
    /// Возвращает `Ok(true)` если прогресс был сделан, `Ok(false)` если ждёт внешнего события.
    pub fn try_advance(&mut self, ctx: &Context) -> Result<bool, String> {
        match self.phase.take() {
            None => {
                // Нет активной фазы — начинаем новый шаг.
                self.start_next_step(ctx)
            }
            Some(phase) => self.continue_phase(ctx, phase),
        }
    }

    /// Начать выполнение следующего шага сценария.
    fn start_next_step(&mut self, ctx: &Context) -> Result<bool, String> {
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
            let verdicts: Vec<QaVerdict> = self
                .state
                .results
                .iter()
                .map(|r| r.verdict.clone())
                .collect();
            self.state.status = QaStatus::Completed { verdicts };
            return Ok(false);
        }

        let _step = &scenario.steps[current_step_index];
        let step_num = current_step_index + 1;
        self.state.status = QaStatus::Running {
            current_step: current_step_index,
            total_steps: scenario.steps.len(),
        };

        // === OBSERVE: запросить скриншот ДО действия ===
        self.state.screenshot_label = Some(format!("step_{step_num}_before"));
        ctx.request_repaint();
        self.phase = Some(QaStepPhase::WaitingBefore { step_index: current_step_index });
        Ok(true)
    }

    /// Продолжить выполнение текущей фазы.
    fn continue_phase(&mut self, ctx: &Context, phase: QaStepPhase) -> Result<bool, String> {
        match phase {
            QaStepPhase::WaitingBefore { step_index } => {
                // Проверяем, готов ли скриншот.
                match self.take_screenshot_result() {
                    Some(before_path) => {
                        // === THINK: спросить у VLM ===
                        let scenario = self.state.scenario.as_ref().unwrap();
                        let step = &scenario.steps[step_index];

                        if self.vlm_client.is_configured() {
                            match self.vlm_client.ask_action(&before_path, &step.instruction) {
                                Ok(action) => {
                                    // === ACT: выполнить действие ===
                                    let before_rev = self.get_current_rev()?;
                                    match self.actuator.execute(&action) {
                                        Ok(()) => {
                                            // Подождать少し, чтобы ревизия обновилась.
                                            std::thread::sleep(std::time::Duration::from_millis(300));
                                            let after_rev = self.get_current_rev()?;

                                            // === OBSERVE: запросить скриншот ПОСЛЕ действия ===
                                            let step_num = step_index + 1;
                                            self.state.screenshot_label =
                                                Some(format!("step_{step_num}_after"));
                                            ctx.request_repaint();
                                            self.phase = Some(QaStepPhase::WaitingAfter {
                                                step_index,
                                                before_path,
                                                action,
                                                before_rev: after_rev,
                                            });
                                            Ok(true)
                                        }
                                        Err(e) => {
                                            let err_str = e.to_string();
                                            self.state.results.push(
                                                crate::app::qa::scenario::QaStepResult {
                                                    step_index,
                                                    instruction: step.instruction.clone(),
                                                    expect: step.expect.clone(),
                                                    action: Some(action.clone()),
                                                    verdict: QaVerdict::Failure {
                                                        reason: format!("Ошибка выполнения действия: {err_str}"),
                                                    },
                                                    screenshot_before: Some(before_path),
                                                    screenshot_after: None,
                                                    comment: format!("Ошибка выполнения: {err_str}"),
                                                },
                                            );
                                            self.state.status = QaStatus::Failed {
                                                step_index,
                                                reason: err_str.clone(),
                                            };
                                            return Err(format!("Ошибка выполнения действия: {err_str}"));
                                        }
                                    }
                                }
                                Err(e) => {
                                    let err_str = e.to_string();
                                    self.state.results.push(
                                        crate::app::qa::scenario::QaStepResult {
                                            step_index,
                                            instruction: step.instruction.clone(),
                                            expect: step.expect.clone(),
                                            action: None,
                                            verdict: QaVerdict::Failure {
                                                reason: format!("VLM ошибка: {err_str}"),
                                            },
                                            screenshot_before: Some(before_path),
                                            screenshot_after: None,
                                            comment: format!("VLM ошибка: {err_str}"),
                                        },
                                    );
                                    self.state.status = QaStatus::Failed {
                                        step_index,
                                        reason: err_str.clone(),
                                    };
                                    return Err(format!("VLM ошибка: {err_str}"));
                                }
                            }
                        } else {
                            // VLM не сконфигурирован.
                            self.state.results.push(
                                crate::app::qa::scenario::QaStepResult {
                                    step_index,
                                    instruction: step.instruction.clone(),
                                    expect: step.expect.clone(),
                                    action: None,
                                    verdict: QaVerdict::Failure {
                                        reason: "VLM не сконфигурирован (нет endpoint или модели)".to_owned(),
                                    },
                                    screenshot_before: Some(before_path),
                                    screenshot_after: None,
                                    comment: "VLM не сконфигурирован".to_owned(),
                                },
                            );
                            self.state.status = QaStatus::Failed {
                                step_index,
                                reason: "VLM не сконфигурирован".to_owned(),
                            };
                            return Err("VLM не сконфигурирован".to_owned());
                        }
                    }
                    None => {
                        // Скриншот ещё не готов — ждём следующего кадра.
                        self.phase = Some(QaStepPhase::WaitingBefore { step_index });
                        Ok(false)
                    }
                }
            }
            QaStepPhase::WaitingAfter {
                step_index,
                before_path,
                action,
                before_rev,
            } => {
                // Проверяем, готов ли скриншот после действия.
                match self.take_screenshot_result() {
                    Some(after_path) => {
                        // === VERIFY: проверить результат ===
                        let scenario = self.state.scenario.as_ref().unwrap();
                        let step = &scenario.steps[step_index];

                        let verdict = self.verifier.verify(
                            step,
                            before_rev,
                            before_rev + 1, // ревизия должна増литься после действия
                            &before_path,
                            &after_path,
                        );

                        self.state.results.push(
                            crate::app::qa::scenario::QaStepResult {
                                step_index,
                                instruction: step.instruction.clone(),
                                expect: step.expect.clone(),
                                action: Some(action),
                                verdict: verdict.clone(),
                                screenshot_before: Some(before_path),
                                screenshot_after: Some(after_path),
                                comment: format!("Ревизия до={before_rev}"),
                            },
                        );

                        match &verdict {
                            QaVerdict::Success => {
                                let next_step = step_index + 1;
                                if next_step >= scenario.steps.len() {
                                    let verdicts: Vec<QaVerdict> = self
                                        .state
                                        .results
                                        .iter()
                                        .map(|r| r.verdict.clone())
                                        .collect();
                                    self.state.status = QaStatus::Completed { verdicts };
                                    return Ok(false);
                                }
                                self.state.status = QaStatus::Running {
                                    current_step: next_step,
                                    total_steps: scenario.steps.len(),
                                };
                                // Начинаем следующий шаг — запрашиваем скриншот ДО.
                                self.state.screenshot_label = Some(format!("step_{next_step}_before"));
                                ctx.request_repaint();
                                self.phase = Some(QaStepPhase::WaitingBefore { step_index: next_step });
                                Ok(true)
                            }
                            QaVerdict::Failure { reason } => {
                                self.state.status = QaStatus::Failed {
                                    step_index,
                                    reason: reason.clone(),
                                };
                                return Err(format!("Шаг {} не пройден: {reason}", step_index + 1));
                            }
                            QaVerdict::Uncertain { reason } => {
                                self.state.status = QaStatus::Failed {
                                    step_index,
                                    reason: reason.clone(),
                                };
                                return Err(format!("Шаг {} неопределён: {reason}", step_index + 1));
                            }
                        }
                    }
                    None => {
                        // Скриншот ещё не готов — ждём следующего кадра.
                        self.phase = Some(QaStepPhase::WaitingAfter {
                            step_index,
                            before_path,
                            action,
                            before_rev,
                        });
                        Ok(false)
                    }
                }
            }
        }
    }

    /// Загрузить сценарий из YAML-строки.
    pub fn load_scenario(&mut self, yaml: &str) -> Result<(), String> {
        let scenario = QaScenario::from_str(yaml)?;
        self.state.scenario = Some(scenario);
        self.state.status = QaStatus::Idle;
        self.state.results.clear();
        self.phase = None;
        Ok(())
    }

    /// Загрузить сценарий из файла.
    pub fn load_scenario_from_file(&mut self, path: &PathBuf) -> Result<(), String> {
        let scenario = QaScenario::from_file(path)?;
        self.state.scenario = Some(scenario);
        self.state.status = QaStatus::Idle;
        self.state.results.clear();
        self.phase = None;
        Ok(())
    }

    /// Получить готовый результат скриншота, если есть.
    fn take_screenshot_result(&mut self) -> Option<PathBuf> {
        let mut guard = self.state.screenshot_result.lock().unwrap();
        if let Some(res) = guard.take() {
            self.state.screenshot_label = None;
            Some(res.map_err(|e| {
                eprintln!("Ошибка сохранения скриншота: {e}");
            }).unwrap_or_else(|_| PathBuf::new()))
        } else {
            None
        }
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

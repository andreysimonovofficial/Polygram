//! Модуль инфраструктуры QA-агента: Observe → Think → Act → Verify.
//!
//! Предоставляет:
//! - DSL для описания сценариев тестирования (scenario.rs)
//! - Оркестратор цикла выполнения сценариев (runner.rs)
//! - Клиент VLM для анализа скриншотов и принятия решений (vision.rs)
//! - Исполнитель действий через HTTP API (actuator.rs)
//! - Верификация результатов до/после действия (verifier.rs)
//! - Формирование отчётов заказчику (report.rs)

pub use runner::QaAgentState;
pub use runner::QaRunner;
pub use scenario::QaAction;
pub use scenario::QaScenario;
pub use scenario::QaStep;
pub use scenario::QaStatus;
pub use scenario::QaVerdict;
pub use verifier::Verifier;
pub use vision::VlmClient;
pub use vision::VlmConfig;

pub mod actuator;
pub mod report;
pub mod runner;
pub mod scenario;
pub mod verifier;
pub mod vision;

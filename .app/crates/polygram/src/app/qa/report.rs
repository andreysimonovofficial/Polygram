//! Формирование отчётов заказчику о выполнении шагов QA.
//!
//! Формат отчёта:
//! «Я нажал на кнопку X. Смотри, что получилось: (скриншот). Это то, что ты ожидал?»
//! «Да, уведомление появилось» / «Нет, уведомления нет».

use crate::app::qa::scenario::{QaStepResult, QaVerdict};

/// Отчёт о выполнении одного шага.
#[derive(Debug, Clone)]
pub struct QaStepReport {
    pub step_index: usize,
    pub instruction: String,
    pub action_description: String,
    pub screenshot_path: Option<String>,
    pub verdict: QaVerdict,
    pub customer_comment: String,
}

/// Генератор отчётов QA.
pub struct ReportGenerator;

impl ReportGenerator {
    /// Создать отчёт о шаге для заказчика.
    pub fn step_report(result: &QaStepResult) -> QaStepReport {
        let action_description = match &result.action {
            Some(action) => match action {
                crate::app::qa::scenario::QaAction::Click { x, y } => {
                    format!("Я нажал на элемент в координатах ({x:.2}, {y:.2})")
                }
                crate::app::qa::scenario::QaAction::Drag {
                    from_x,
                    from_y,
                    to_x,
                    to_y,
                } => format!(
                    "Я перетащил элемент из ({from_x:.2}, {from_y:.2}) в ({to_x:.2}, {to_y:.2})"
                ),
                crate::app::qa::scenario::QaAction::KeyDown { key } => {
                    format!("Я зажал клавишу {key}")
                }
                crate::app::qa::scenario::QaAction::KeyUp { key } => {
                    format!("Я отпустил клавишу {key}")
                }
                crate::app::qa::scenario::QaAction::Wheel {
                    delta_x,
                    delta_y,
                } => format!(
                    "Я покрутил колёсико мыши (delta_x={delta_x}, delta_y={delta_y})"
                ),
                crate::app::qa::scenario::QaAction::Wait { ms } => {
                    format!("Я подождал {ms} мс")
                }
            },
            None => "Действие не выполнено".to_owned(),
        };

        let screenshot_path = result
            .screenshot_after
            .as_ref()
            .map(|p| p.display().to_string());

        let customer_comment = match &result.verdict {
            QaVerdict::Success => {
                format!(
                    "{}.\nСмотри, что получилось: {}\nЭто то, что ты ожидал?\nДа, ожидание подтверждено: {}.",
                    action_description,
                    screenshot_path
                        .as_deref()
                        .unwrap_or("скриншот не сохранён"),
                    result.expect
                )
            }
            QaVerdict::Failure { reason } => {
                format!(
                    "{}.\nСмотри, что получилось: {}\nЭто то, что ты ожидал?\nНет, ожидание не подтверждено: {}.",
                    action_description,
                    screenshot_path
                        .as_deref()
                        .unwrap_or("скриншот не сохранён"),
                    reason
                )
            }
            QaVerdict::Uncertain { reason } => {
                format!(
                    "{}.\nСмотри, что получилось: {}\nЭто то, что ты ожидал?\nНе удалось определить однозначно: {}.",
                    action_description,
                    screenshot_path
                        .as_deref()
                        .unwrap_or("скриншот не сохранён"),
                    reason
                )
            }
        };

        QaStepReport {
            step_index: result.step_index,
            instruction: result.instruction.clone(),
            action_description,
            screenshot_path,
            verdict: result.verdict.clone(),
            customer_comment,
        }
    }

    /// Создать полный отчёт о сценарии.
    pub fn scenario_report(
        scenario_name: &str,
        results: &[QaStepResult],
    ) -> String {
        let mut lines = Vec::new();
        lines.push(format!("Отчёт по сценарию QA: {scenario_name}"));
        lines.push(String::new());

        let mut success_count = 0;
        let mut failure_count = 0;

        for result in results {
            let report = Self::step_report(result);
            lines.push(format!("Шаг {}: {}", report.step_index + 1, report.instruction));
            lines.push(format!("  Действие: {}", report.action_description));
            if let Some(path) = &report.screenshot_path {
                lines.push(format!("  Скриншот: {path}"));
            }
            lines.push(format!(
                "  Вердикт: {}",
                match report.verdict {
                    QaVerdict::Success => "Успешно",
                    QaVerdict::Failure { .. } => "Ошибка",
                    QaVerdict::Uncertain { .. } => "Неопределённо",
                }
            ));
            lines.push(format!("  Комментарий: {}", report.customer_comment));
            lines.push(String::new());

            match result.verdict {
                QaVerdict::Success => success_count += 1,
                QaVerdict::Failure { .. } => failure_count += 1,
                QaVerdict::Uncertain { .. } => {}
            }
        }

        lines.push(format!(
            "Итого: {} шагов выполнено успешно, {} с ошибкой.",
            success_count, failure_count
        ));

        lines.join("\n")
    }

    /// Метка вердикта для отчёта.
    pub fn verdict_label(verdict: &QaVerdict) -> &str {
        match verdict {
            QaVerdict::Success => "Успешно",
            QaVerdict::Failure { .. } => "Ошибка",
            QaVerdict::Uncertain { .. } => "Неопределённо",
        }
    }
}

//! Регрессионный раннер сценариев для ядра Polygram.
//! Сценарий — JSONL: одна строка = один шаг.
//!   {"apply": <Command>}   — применить команду к ядру.
//!   {"expect": <Expectation>} — проверить текущее состояние ядра.
//!   Можно сочетать apply+expect в одной строке.
//! Все проверки идут против чистого ядра (`Engine`) — без окна egui, мыши и агента.

use polygram_core::*;
use serde::Deserialize;

/// Один шаг сценария.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub apply: Option<Command>,
    #[serde(default)]
    pub expect: Option<Expectation>,
}

/// Проверки после применения команды (или против текущего состояния).
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expectation {
    #[serde(default)]
    pub rev: Option<u64>,
    #[serde(default)]
    pub created: Option<String>,
    #[serde(default)]
    pub at: Option<[i32; 2]>,
    #[serde(default)]
    pub objects: Option<usize>,
    #[serde(default)]
    pub edges: Option<usize>,
    #[serde(default)]
    pub statuses: Option<Vec<StatusExpect>>,
    #[serde(default)]
    pub values: Option<Vec<ValueExpect>>,
    #[serde(default)]
    pub supplied: Option<Vec<ValueExpect>>,
    #[serde(default)]
    pub parents: Option<Vec<ParentExpect>>,
    #[serde(default)]
    pub bottlenecks: Option<Vec<BottleneckExpect>>,
    #[serde(default)]
    pub no_overlaps: Option<bool>,
    /// Ожидаемый код ошибки от последней команды `apply` (например "duplicate_id").
    #[serde(default)]
    pub error: Option<String>,
    /// Ожидать, что последняя команда `apply` выполнилась без ошибки.
    #[serde(default)]
    pub error_absent: Option<bool>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusExpect {
    pub object: String,
    pub entry: String,
    pub code: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValueExpect {
    pub object: String,
    pub entry: String,
    pub value: MetricValue,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParentExpect {
    pub object: String,
    #[serde(default)]
    pub parent: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BottleneckExpect {
    pub object: String,
    pub entry: String,
    #[serde(default)]
    pub shortfall: Option<f64>,
    #[serde(default)]
    pub consumers: Option<u32>,
}

pub struct StepResult {
    pub index: usize,
    pub label: Option<String>,
    pub ok: bool,
    pub errors: Vec<String>,
}

impl StepResult {
    pub fn summary(&self) -> String {
        if self.ok {
            format!("шаг {} ok", self.index)
        } else {
            format!("шаг {} ПРОВАЛ: {}", self.index, self.errors.join("; "))
        }
    }
}

/// Прогнать один сценарий (JSONL-строка) на чистом ядре без окна.
pub fn run_scenario(source: &str) -> Vec<StepResult> {
    let mut engine = Engine::new();
    let mut results: Vec<StepResult> = Vec::new();
    let mut last_apply: Option<Result<Applied>> = None;

    for (index, raw) in source.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }

        let step: Step = match serde_json::from_str(line) {
            Ok(s) => s,
            Err(e) => {
                results.push(StepResult {
                    index,
                    label: None,
                    ok: false,
                    errors: vec![format!("ошибка парсинга строки: {e}")],
                });
                continue;
            }
        };
        let Step {
            label,
            apply,
            expect,
        } = step;

        let mut errors: Vec<String> = Vec::new();

        // Применяем команду, если есть.
        if let Some(cmd) = apply {
            last_apply = Some(engine.apply(Scope::Root, cmd));
        }

        // Проверяем ожидания.
        if let Some(exp) = &expect {
            check_expectations(&engine, &last_apply, exp, &mut errors);
        }

        results.push(StepResult {
            index,
            label,
            ok: errors.is_empty(),
            errors,
        });
    }

    results
}

fn check_expectations(
    engine: &Engine,
    last_apply: &Option<Result<Applied>>,
    exp: &Expectation,
    errors: &mut Vec<String>,
) {
    let apply_ok = last_apply.as_ref().and_then(|r| r.as_ref().ok());

    // Ошибка apply.
    if let Some(expected_code) = &exp.error {
        match last_apply {
            Some(Err(e)) => {
                if e.code() != expected_code.as_str() {
                    errors.push(format!(
                        "apply: ожидалась ошибка «{}», получена «{}» ({})",
                        expected_code,
                        e.code(),
                        e
                    ));
                }
            }
            Some(Ok(_)) => {
                errors.push(format!(
                    "apply: ожидалась ошибка «{}», но команда выполнена успешно",
                    expected_code
                ));
            }
            None => {
                errors.push(format!(
                    "apply: ожидалась ошибка «{}», но команда не применялась",
                    expected_code
                ));
            }
        }
    }
    if exp.error_absent == Some(true) {
        match last_apply {
            Some(Err(e)) => {
                errors.push(format!("apply: ошибка не ожидалась, получена «{}»: {}", e.code(), e));
            }
            None => {
                errors.push("apply: ошибка не ожидалась, но команда не применялась".to_string());
            }
            Some(Ok(_)) => {}
        }
    }

    // Ревизия — источник истины из движка.
    if let Some(expected) = exp.rev {
        if engine.rev() != expected {
            errors.push(format!("rev: ожидалось {}, получено {}", expected, engine.rev()));
        }
    }

    // created / at — только после успешного apply.
    if let Some(applied) = apply_ok {
        if let Some(expected) = &exp.created {
            match &applied.created {
                Some(id) if id == expected => {}
                Some(id) => {
                    errors.push(format!("created: ожидалось «{}», получено «{}»", expected, id));
                }
                None => {
                    errors.push(format!(
                        "created: ожидалось «{}», но команда не вернула идентификатор",
                        expected
                    ));
                }
            }
        }
        if let Some(expected) = exp.at {
            match applied.at {
                Some(at) if at == expected => {}
                Some(at) => {
                    errors.push(format!("at: ожидалось {:?}, получено {:?}", expected, at));
                }
                None => {
                    errors.push(format!(
                        "at: ожидалось {:?}, но команда не вернула позицию",
                        expected
                    ));
                }
            }
        }
    }

    if let Some(expected) = exp.objects {
        let actual = engine.stats().objects;
        if actual != expected {
            errors.push(format!("objects: ожидалось {}, получено {}", expected, actual));
        }
    }

    if let Some(expected) = exp.edges {
        let actual = engine.stats().edges;
        if actual != expected {
            errors.push(format!("edges: ожидалось {}, получено {}", expected, actual));
        }
    }

    if exp.no_overlaps.unwrap_or(false) {
        let g = engine.geometry();
        for (i, a) in g.objects.iter().enumerate() {
            for b in &g.objects[i + 1..] {
                if a.rect.overlaps(&b.rect) {
                    errors.push(format!("перекрытие: «{}» и «{}»", a.id, b.id));
                }
            }
        }
    }

    if let Some(expected_statuses) = &exp.statuses {
        for se in expected_statuses {
            let ov = match engine.object_view(&se.object) {
                Some(v) => v,
                None => {
                    errors.push(format!("status: объект «{}» не найден", se.object));
                    continue;
                }
            };
            let entry = ov
                .requests
                .iter()
                .chain(ov.outputs.iter())
                .find(|m| m.id == se.entry);
            match entry {
                Some(m) => {
                    let expected_code = match se.code.as_str() {
                        "ok" => StatusCode::Ok,
                        "surplus" => StatusCode::Surplus,
                        "in_progress" => StatusCode::InProgress,
                        "expected" => StatusCode::Expected,
                        other => {
                            errors.push(format!(
                                "status: неизвестный код «{other}» для {}/{}",
                                se.object, se.entry
                            ));
                            continue;
                        }
                    };
                    if m.status != expected_code {
                        errors.push(format!(
                            "status: {}/{} ожидался {:?}, получен {:?}",
                            se.object, se.entry, expected_code, m.status
                        ));
                    }
                }
                None => {
                    errors.push(format!(
                        "status: запись «{}» не найдена на объекте «{}»",
                        se.entry, se.object
                    ));
                }
            }
        }
    }

    check_values(engine, &exp.values, "value", |m: &MetricView| m.value.clone(), errors);
    check_values(
        engine,
        &exp.supplied,
        "supplied",
        |m: &MetricView| m.supplied.clone(),
        errors,
    );

    if let Some(expected_parents) = &exp.parents {
        let g = engine.geometry();
        for pe in expected_parents {
            let obj = g.objects.iter().find(|o| o.id == pe.object);
            match obj {
                Some(o) => {
                    if o.parent != pe.parent {
                        errors.push(format!(
                            "parent: «{}» ожидался родитель {:?}, получен {:?}",
                            pe.object, pe.parent, o.parent
                        ));
                    }
                }
                None => {
                    errors.push(format!("parent: объект «{}» не найден", pe.object));
                }
            }
        }
    }

    if let Some(expected_bottlenecks) = &exp.bottlenecks {
        let actual = engine.bottlenecks(expected_bottlenecks.len());
        let ids: Vec<String> = engine.object_ids();
        if actual.len() != expected_bottlenecks.len() {
            errors.push(format!(
                "bottlenecks: ожидалось {}, получено {}",
                expected_bottlenecks.len(),
                actual.len()
            ));
        }
        for (i, be) in expected_bottlenecks.iter().enumerate() {
            if i >= actual.len() {
                break;
            }
            let b = &actual[i];
            let obj_id = ids
                .get(b.object as usize)
                .cloned()
                .unwrap_or_else(|| format!("индекс {}", b.object));
            if be.object != obj_id {
                errors.push(format!(
                    "bottleneck{}: объект ожидался «{}», получен «{}»",
                    i, be.object, obj_id
                ));
            }
            if let Some(sf) = be.shortfall {
                if (b.shortfall - sf).abs() > 1e-6 {
                    errors.push(format!(
                        "bottleneck{}: shortfall ожидался {}, получено {}",
                        i, sf, b.shortfall
                    ));
                }
            }
            if let Some(c) = be.consumers {
                if b.consumers != c {
                    errors.push(format!(
                        "bottleneck{}: consumers ожидалось {}, получено {}",
                        i, c, b.consumers
                    ));
                }
            }
        }
    }
}

fn check_values<F>(
    engine: &Engine,
    expects: &Option<Vec<ValueExpect>>,
    field: &str,
    pick: F,
    errors: &mut Vec<String>,
) where
    F: Fn(&MetricView) -> MetricValue,
{
    if let Some(expected_values) = expects {
        for ve in expected_values {
            let ov = match engine.object_view(&ve.object) {
                Some(v) => v,
                None => {
                    errors.push(format!("{}: объект «{}» не найден", field, ve.object));
                    continue;
                }
            };
            let entry = ov
                .requests
                .iter()
                .chain(ov.outputs.iter())
                .find(|m| m.id == ve.entry);
            match entry {
                Some(m) => {
                    let actual = pick(m);
                    if actual != ve.value {
                        errors.push(format!(
                            "{}: {}/{} ожидалось {}, получено {}",
                            field, ve.object, ve.entry, ve.value, actual
                        ));
                    }
                }
                None => {
                    errors.push(format!(
                        "{}: запись «{}» не найдена на объекте «{}»",
                        field, ve.entry, ve.object
                    ));
                }
            }
        }
    }
}

/// Удобная обёртка для тестов: паникует с подробностями, если какой-то шаг упал.
pub fn assert_scenario(source: &str) {
    let results = run_scenario(source);
    let mut failures = Vec::new();
    for r in &results {
        if !r.ok {
            failures.push(r.summary());
        }
    }
    if !failures.is_empty() {
        panic!("Сценарий упал:\n{}", failures.join("\n"));
    }
}

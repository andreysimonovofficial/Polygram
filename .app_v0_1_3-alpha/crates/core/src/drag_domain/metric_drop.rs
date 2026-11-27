//! Расчёт команд при дропе показателя (метрики) между объектами.

use crate::command::Command;
use crate::geometry::Geometry;
use crate::types::Direction;
use crate::{MetricValue};

/// Вход для расчёта команд при дропе показателя (метрики).
pub struct MetricDropContext {
    /// Геометрия с наложенными предсказаниями.
    pub geometry: Geometry,
    /// Объект-источник показателя.
    pub source_object: String,
    /// Направление показателя у источника.
    pub source_dir: Direction,
    /// Имя показываемой записи.
    pub entry_name: String,
    /// Целевой объект.
    pub target_id: String,
}

/// Рассчитать команды при дропе показателя: добавить запись в цель (если нет) и создать связь.
pub fn compute_metric_drop(ctx: &MetricDropContext) -> Vec<Command> {
    let MetricDropContext {
        geometry,
        source_object,
        source_dir,
        entry_name,
        target_id,
    } = ctx;

    if target_id == source_object {
        return Vec::new();
    }

    let target = match geometry.objects.iter().find(|o| o.id == *target_id) {
        Some(t) if t.kind == crate::types::NodeKind::Process => t,
        _ => return Vec::new(),
    };

    let target_dir = match source_dir {
        Direction::Request => Direction::Result,
        Direction::Result => Direction::Request,
    };

    let entries = match target_dir {
        Direction::Request => &target.requests,
        Direction::Result => &target.results,
    };

    let already_has = entries.iter().any(|e| e.name == *entry_name);
    let mut cmds: Vec<Command> = Vec::new();

    if !already_has {
        cmds.push(Command::AddMetricEntry {
            object: target_id.clone(),
            dir: target_dir,
            id: Some(entry_name.clone()),
            metric_def: entry_name.clone(),
            name: entry_name.clone(),
            value: MetricValue::Null,
            required: MetricValue::Null,
            max: None,
            fact: MetricValue::Null,
        });
    }

    let (from_object, from_entry, to_object, to_entry) = match source_dir {
        Direction::Result => (
            source_object.clone(),
            entry_name.clone(),
            target_id.clone(),
            entry_name.clone(),
        ),
        Direction::Request => (
            target_id.clone(),
            entry_name.clone(),
            source_object.clone(),
            entry_name.clone(),
        ),
    };

    cmds.push(Command::Connect {
        from_object,
        from_entry,
        to_object,
        to_entry,
        throughput: 100.0,
    });

    cmds
}

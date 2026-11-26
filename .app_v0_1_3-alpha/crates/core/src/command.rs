//! «Терминал»: единый язык команд.
//! Команды сериализуются в JSON (поле `cmd`), поэтому их можно подавать из файла или по сети.

use crate::types::*;
use serde::{Deserialize, Serialize};

fn null_value() -> MetricValue {
    MetricValue::Null
}

fn default_throughput() -> f64 {
    100.0
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Command {
    AddObject {
        #[serde(default)]
        id: Option<String>,
        name: String,
        #[serde(default)]
        kind: NodeKind,
        #[serde(default)]
        author: Option<String>,
        #[serde(default)]
        at: Option<[i32; 2]>,
        #[serde(default)]
        parent: Option<String>,
        /// Направление контейнера (только для kind = Container).
        #[serde(default)]
        direction: Option<ContainerDirection>,
    },
    RemoveObject { object: String },
    RenameObject { object: String, name: String },
    MoveObject { object: String, at: [i32; 2] },
    SetObjectParent { object: String, #[serde(default)] parent: Option<String> },

    SetObjectIcon { object: String, icon: String },

    AddMetricDef {
        #[serde(default)]
        id: Option<String>,
        name: String,
        #[serde(default)]
        kind: MetricKind,
        #[serde(default)]
        unit: String,
        #[serde(default)]
        api: Option<MetricApi>,
    },
    RemoveMetricDef { metric_def: String },
    RenameMetricDef { metric_def: String, name: String },
    SetMetricDefUnit { metric_def: String, unit: String },

    AddMetricEntry {
        object: String,
        dir: Direction,
        #[serde(default)]
        id: Option<String>,
        metric_def: String,
        name: String,
        #[serde(default = "null_value")]
        value: MetricValue,
        #[serde(default = "null_value")]
        required: MetricValue,
        #[serde(default)]
        max: Option<MetricValue>,
        #[serde(default = "null_value")]
        fact: MetricValue,
    },
    RemoveMetricEntry { object: String, entry: String },
    ReorderMetricEntry {
        object: String,
        dir: Direction,
        from_ix: usize,
        to_ix: usize,
    },
    SetValue { object: String, entry: String, value: MetricValue },
    SetRequired { object: String, entry: String, required: MetricValue },
    SetMax { object: String, entry: String, max: Option<MetricValue> },
    SetFact { object: String, entry: String, fact: MetricValue },
    SetInProcess {
        object: String,
        entry: String,
        #[serde(default)]
        in_process: bool,
    },

    Connect {
        from_object: String,
        from_entry: String,
        to_object: String,
        to_entry: String,
        #[serde(default = "default_throughput")]
        throughput: f64,
    },
    Disconnect {
        from_object: String,
        from_entry: String,
        to_object: String,
        to_entry: String,
    },

    AutoLayout,
    FitCamera,
    AutoScreenshot { enabled: bool },

    RestartServer,
    ChangePort { port: u16 },
    Shutdown,
    ReloadConfig,

    VirtualMouseMove { x: i32, y: i32 },
    VirtualMouseDown { button: u8 },
    VirtualMouseUp { button: u8 },
    VirtualKeyboardType { text: String },
    VirtualDrawShape { kind: u8, x2: i32, y2: i32 },
    VirtualMouseCurve { x: i32, y: i32, cx: i32, cy: i32 },
    VirtualMousePath { points: Vec<(i32, i32)> },
    VirtualMouseDownMiddle,
    VirtualMouseUpMiddle,
    VirtualKeyDown { key: String },
    VirtualKeyUp { key: String },
    VirtualWheel { delta_x: f32, delta_y: f32 },
    VirtualDrag { object_id: String, to_x: i32, to_y: i32 },
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Applied {
    pub rev: u64,
    pub created: Option<String>,
    pub at: Option<[i32; 2]>,
}

pub fn default_unit(kind: MetricKind) -> &'static str {
    match kind {
        MetricKind::Number => "ед.",
        MetricKind::Time => "чч:мм",
        MetricKind::Date => "дд.мм.гггг",
        MetricKind::Deadline => "дд.мм.гггг чч:мм",
        MetricKind::Percentage => "%",
        MetricKind::Binary => "",
        MetricKind::Duration => "чч:мм",
        MetricKind::Money => "₽",
        MetricKind::Text => "",
        MetricKind::Formula => "",
        MetricKind::Timestamp => "",
    }
}

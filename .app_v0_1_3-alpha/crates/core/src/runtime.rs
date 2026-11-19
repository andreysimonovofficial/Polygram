//! Рантайм: снимок состояния для GUI.

use crate::{values::DynamicState, Geometry, MetricDefView};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Published {
    pub rev: u64,
    pub dynamic: DynamicState,
    pub geometry: Geometry,
    pub metric_defs: Vec<MetricDefView>,
    pub camera_fit_requested: bool,
    pub auto_screenshot: bool,
}

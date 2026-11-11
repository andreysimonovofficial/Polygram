use polygram_core::{Direction, MetricKind};
use polygram_core::types::MAX_UI_METRICS_PER_SIDE;
use chrono::{NaiveDate, NaiveTime};
use std::collections::HashMap;

/// API binding settings being edited for a single metric entry.
#[derive(Clone, Default)]
pub struct EntryApiBinding {
    pub connector: String,
    pub cell: String,
    pub interval_secs: u64,
}

pub struct DialogState {
    pub rename_target: Option<String>,
    pub rename_input: String,
    pub edit_target: Option<String>,
    pub edit_input: String,
    pub add_metric_target: Option<(String, Direction)>,
    pub add_metric_name: String,
    pub add_metric_kind: MetricKind,
    pub add_metric_value: String,
    pub add_metric_date: Option<NaiveDate>,
    pub add_metric_date_popup_open: bool,
    pub add_metric_time: Option<NaiveTime>,
    pub add_metric_unit: String,
    pub add_metric_api_connector: String,
    pub add_metric_api_cell: String,
    pub container_menu_target: Option<(String, Direction)>,
    pub container_menu_edits: Vec<(String, String, String)>,
    /// Per-entry API binding settings being edited in the container menu dialog.
    pub container_menu_api_settings: HashMap<String, EntryApiBinding>,
    /// Entry name whose settings block is currently expanded.
    pub container_menu_api_expanded: Option<String>,
    /// Объекты, ожидающие подтверждения удаления ("Удалить?").
    pub delete_confirm: Option<Vec<String>>,
}

impl DialogState {
    pub fn new() -> Self {
        Self {
            rename_target: None,
            rename_input: String::new(),
            edit_target: None,
            edit_input: String::new(),
            add_metric_target: None,
            add_metric_name: String::new(),
            add_metric_kind: MetricKind::Number,
            add_metric_value: String::new(),
            add_metric_date: None,
            add_metric_date_popup_open: false,
            add_metric_time: None,
            add_metric_unit: String::new(),
            add_metric_api_connector: String::new(),
            add_metric_api_cell: String::new(),
            container_menu_target: None,
            container_menu_edits: Vec::new(),
            container_menu_api_settings: HashMap::new(),
            container_menu_api_expanded: None,
            delete_confirm: None,
        }
    }
}

impl DialogState {
    pub fn reset_rename(&mut self) {
        self.rename_target = None;
        self.rename_input.clear();
    }

    pub fn reset_edit(&mut self) {
        self.edit_target = None;
        self.edit_input.clear();
    }

    pub fn reset_add_metric(&mut self) {
        self.add_metric_target = None;
        self.add_metric_name.clear();
        self.add_metric_value.clear();
        self.add_metric_date = None;
        self.add_metric_date_popup_open = false;
        self.add_metric_time = None;
        self.add_metric_unit.clear();
        self.add_metric_api_connector.clear();
        self.add_metric_api_cell.clear();
        self.add_metric_kind = MetricKind::Number;
    }

    pub fn reset_container_menu(&mut self) {
        self.container_menu_target = None;
        self.container_menu_edits.clear();
        self.container_menu_api_settings.clear();
        self.container_menu_api_expanded = None;
    }

    pub fn can_add_metric(&self, object_id: &str, dir: Direction, published: &polygram_core::Published) -> bool {
        let n = match dir {
            Direction::Request => published
                .geometry
                .objects
                .iter()
                .find(|o| o.id == *object_id)
                .map(|o| o.requests.len())
                .unwrap_or(0),
            Direction::Output => published
                .geometry
                .objects
                .iter()
                .find(|o| o.id == *object_id)
                .map(|o| o.outputs.len())
                .unwrap_or(0),
        };
        n < MAX_UI_METRICS_PER_SIDE
    }

    pub fn add_metric_defaults(&mut self, object_id: String, dir: Direction, n: usize) {
        self.add_metric_target = Some((object_id, dir));
        self.add_metric_name = match dir {
            Direction::Request => format!("request{}", n + 1),
            Direction::Output => format!("out{}", n + 1),
        };
        self.add_metric_kind = MetricKind::Number;
        self.add_metric_value = "0".to_owned();
        self.add_metric_date = None;
        self.add_metric_date_popup_open = false;
        self.add_metric_time = None;
        self.add_metric_unit = String::new();
    }

    pub fn container_menu_entry_names(&self) -> Vec<String> {
        self.container_menu_edits.iter().map(|(n, _, _)| n.clone()).collect()
    }

    pub fn ensure_container_menu_edits(&mut self, entries: &[polygram_core::geometry::EntryGeom]) {
        if self.container_menu_edits.is_empty() {
            for entry in entries {
                self.container_menu_edits.push((
                    entry.name.clone(),
                    format!("{:.0}", entry.value),
                    entry.unit.clone(),
                ));
            }
        } else {
            let entry_names: std::collections::HashSet<&str> =
                entries.iter().map(|e| e.name.as_str()).collect();
            self.container_menu_edits
                .retain(|(n, _, _)| entry_names.contains(n.as_str()));
            for entry in entries {
                if !self
                    .container_menu_edits
                    .iter()
                    .any(|(n, _, _)| *n == entry.name)
                {
                    self.container_menu_edits.push((
                        entry.name.clone(),
                        format!("{:.0}", entry.value),
                        entry.unit.clone(),
                    ));
                }
            }
        }
    }

    pub fn apply_container_edits(&mut self) -> (Vec<(String, f64)>, Vec<(String, String)>) {
        let value_cmds: Vec<(String, f64)> = self
            .container_menu_edits
            .iter()
            .filter_map(|(entry_name, value_str, _)| {
                let value: f64 = value_str.trim().parse().unwrap_or(0.0);
                Some((entry_name.clone(), value))
            })
            .collect();
        let unit_cmds: Vec<(String, String)> = self
            .container_menu_edits
            .iter()
            .filter_map(|(entry_name, _, unit_str)| {
                let unit = unit_str.trim().to_owned();
                if !unit.is_empty() {
                    Some((entry_name.clone(), unit))
                } else {
                    None
                }
            })
            .collect();
        self.container_menu_edits.clear();
        (value_cmds, unit_cmds)
    }
}

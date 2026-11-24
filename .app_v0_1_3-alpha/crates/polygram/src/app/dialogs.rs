//! Состояние диалогов приложения Polygram.
//! Хранит буферы ввода для добавления показателя, редактирования контейнера,
//! подтверждений удаления и др.

use crate::app::interaction::place_object::PlaceObjectConfirmData;
use chrono::{NaiveDate, NaiveTime};
use polygram_core::{Direction, MetricKind};
use std::collections::HashMap;

/// API binding settings being edited for a single metric entry.
#[derive(Clone, Default)]
pub struct EntryApiBinding {
    pub connector: String,
    pub cell: String,
    pub interval_secs: u64,
}

/// Вкладки полноэкранного окна показателей объекта.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ContainerMenuTab {
    #[default]
    Metrics,
    Chat,
}

/// Состояние диалогов приложения.
#[derive(Clone, Default)]
pub struct DialogState {
    pub rename_target: Option<String>,
    pub rename_input: String,
    pub edit_target: Option<String>,
    pub edit_input: String,
    /// Ожидает подтверждения размещения объекта в объекте («Поместить Объект в Объект»):
    /// (id объекта, имя объекта, вид объекта, id цели, имя цели, вид цели, клетка размещения, стартовая клетка).
    pub place_object_confirm: Option<PlaceObjectConfirmData>,
    pub add_metric_target: Option<(String, Direction)>,
    pub add_metric_process_name: String,
    pub add_metric_name: String,
    pub add_metric_kind: MetricKind,
    pub add_metric_value: String,
    pub add_metric_date: Option<NaiveDate>,
    pub add_metric_date_popup_open: bool,
    pub add_metric_time: Option<NaiveTime>,
    pub add_metric_unit: String,
    pub add_metric_api_connector: String,
    pub add_metric_api_cell: String,
    pub container_menu_target: Option<String>,
    pub container_menu_active_section: Option<Direction>,
    pub container_menu_tab: ContainerMenuTab,
    pub container_menu_section_open: HashMap<Direction, bool>,
    pub container_menu_edits: Vec<(String, Direction, String, String)>,
    pub container_menu_api_settings: HashMap<String, EntryApiBinding>,
    pub container_menu_api_expanded: Option<String>,
    pub delete_confirm: Option<Vec<String>>,
    pub show_metrics_screen: bool,
    pub metrics_target_object: Option<String>,
}

impl DialogState {
    pub fn new() -> Self {
        Self {
            rename_target: None,
            rename_input: String::new(),
            edit_target: None,
            edit_input: String::new(),
            place_object_confirm: None,
            add_metric_target: None,
            add_metric_process_name: String::new(),
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
            container_menu_active_section: None,
            container_menu_tab: ContainerMenuTab::Metrics,
            container_menu_section_open: HashMap::new(),
            container_menu_edits: Vec::new(),
            container_menu_api_settings: HashMap::new(),
            container_menu_api_expanded: None,
            delete_confirm: None,
            show_metrics_screen: false,
            metrics_target_object: None,
        }
    }

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
        self.add_metric_process_name.clear();
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
        self.container_menu_active_section = None;
        self.container_menu_tab = ContainerMenuTab::Metrics;
        self.container_menu_section_open.clear();
        self.container_menu_edits.clear();
        self.container_menu_api_settings.clear();
        self.container_menu_api_expanded = None;
        self.show_metrics_screen = false;
        self.metrics_target_object = None;
    }

    pub fn can_add_metric(&self, object_id: &str, dir: Direction, geometry: &polygram_core::Geometry) -> bool {
        let n = match dir {
            Direction::Request => geometry
                .objects
                .iter()
                .find(|o| o.id == *object_id)
                .map(|o| o.requests.len())
                .unwrap_or(0),
            Direction::Result => geometry
                .objects
                .iter()
                .find(|o| o.id == *object_id)
                .map(|o| o.results.len())
                .unwrap_or(0),
        };
        n < polygram_core::types::MAX_UI_METRICS_PER_SIDE
    }

    pub fn add_metric_defaults(&mut self, object_id: String, dir: Direction, n: usize) {
        self.add_metric_target = Some((object_id, dir));
        self.add_metric_name = match dir {
            Direction::Request => format!("request{}", n + 1),
            Direction::Result => format!("result{}", n + 1),
        };
        self.add_metric_kind = MetricKind::Number;
        self.add_metric_value = "0".to_owned();
        self.add_metric_date = None;
        self.add_metric_date_popup_open = false;
        self.add_metric_time = None;
        self.add_metric_unit = String::new();
    }

    pub fn ensure_container_menu_edits(
        &mut self,
        entries: &[&polygram_core::geometry::EntryGeom],
        dir: Direction,
        published: &polygram_core::runtime::Published,
    ) {
        if self.container_menu_edits.is_empty() {
            for entry in entries {
                let buf_value = match dir {
                    polygram_core::types::Direction::Request => format!(
                        "{:.0}",
                        published.dynamic.required[entry.ix as usize]
                    ),
                    polygram_core::types::Direction::Result => format!("{:.0}", entry.value),
                };
                self.container_menu_edits.push((
                    entry.name.clone(),
                    dir,
                    buf_value,
                    entry.unit.clone(),
                ));
            }
        } else {
            let entry_names: std::collections::HashSet<&str> =
                entries.iter().map(|e| e.name.as_str()).collect();
            self.container_menu_edits
                .retain(|(n, d, _, _)| *d != dir || entry_names.contains(n.as_str()));
            for entry in entries {
                if self
                    .container_menu_edits
                    .iter()
                    .any(|(n, d, _, _)| *n == entry.name && *d == dir)
                {
                    continue;
                }
                let buf_value = match dir {
                    polygram_core::types::Direction::Request => format!(
                        "{:.0}",
                        published.dynamic.required[entry.ix as usize]
                    ),
                    polygram_core::types::Direction::Result => format!("{:.0}", entry.value),
                };
                self.container_menu_edits.push((
                    entry.name.clone(),
                    dir,
                    buf_value,
                    entry.unit.clone(),
                ));
            }
        }
    }

    pub fn find_container_menu_edit(&self, entry_name: &str, dir: Direction) -> Option<usize> {
        self.container_menu_edits
            .iter()
            .position(|(n, d, _, _)| *n == entry_name && *d == dir)
    }

    pub fn apply_container_edits(
        &mut self,
    ) -> (
        Vec<(String, Direction, f64)>,
        Vec<(String, String)>,
    ) {
        let value_cmds: Vec<(String, Direction, f64)> = self
            .container_menu_edits
            .iter()
            .filter_map(|(entry_name, dir, value_str, _)| {
                let value: f64 = value_str.trim().parse().unwrap_or(0.0);
                Some((entry_name.clone(), *dir, value))
            })
            .collect();
        let unit_cmds: Vec<(String, String)> = self
            .container_menu_edits
            .iter()
            .filter_map(|(entry_name, _, _, unit_str)| {
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

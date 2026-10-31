use petgraph::graph::DiGraph;
use rstar::{AABB, RTreeObject};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Metric {
    pub id: String,
    pub name: String,
    pub value: f64,
    #[serde(default)]
    pub required: f64,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default = "default_unit")]
    pub unit: String,
}

fn default_unit() -> String {
    "ед.".to_owned()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub name: String,
    #[serde(default = "default_kind")]
    pub kind: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub is_department: bool,
    #[serde(default)]
    pub is_portal: bool,
    pub x: f32,
    pub y: f32,
    #[serde(default)]
    pub inputs: Vec<Metric>,
    #[serde(default)]
    pub outputs: Vec<Metric>,
    #[serde(default)]
    pub poll_url: String,
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,
}

fn default_kind() -> String {
    "Процесс".to_owned()
}

fn default_poll_interval() -> u64 {
    5
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Edge {
    pub from_node: String,
    pub from_metric: String,
    pub to_node: String,
    pub to_metric: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    #[serde(default = "new_project_id")]
    pub storage_id: String,
    #[serde(default = "default_project_name")]
    pub name: String,
    #[serde(default)]
    pub nodes: Vec<Node>,
    #[serde(default)]
    pub edges: Vec<Edge>,
}

fn default_project_name() -> String {
    "Новый граф".to_owned()
}

fn new_project_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn fresh_storage_id() -> String {
    new_project_id()
}

impl Default for Project {
    fn default() -> Self {
        Self {
            storage_id: "polygram-default-project".to_owned(),
            name: "Маркетинг и продажи".to_owned(),
            nodes: vec![
                Node {
                    id: "PROC_MKT_01".to_owned(),
                    name: "Маркетинг".to_owned(),
                    kind: "Отдел".to_owned(),
                    parent_id: None,
                    is_department: false,
                    is_portal: false,
                    x: 100.0,
                    y: 120.0,
                    inputs: vec![Metric {
                        id: "M_CONTACTS".to_owned(),
                        name: "База контактов".to_owned(),
                        value: 213_560.0,
                        required: 100_000.0,
                        max: None,
                        unit: "ед.".to_owned(),
                    }],
                    outputs: vec![Metric {
                        id: "M_LEADS".to_owned(),
                        name: "Лиды".to_owned(),
                        value: 21_427.0,
                        required: 15_000.0,
                        max: Some(30_000.0),
                        unit: "ед.".to_owned(),
                    }],
                    poll_url: String::new(),
                    poll_interval_secs: 5,
                },
                Node {
                    id: "PROC_SALES_01".to_owned(),
                    name: "Продажи".to_owned(),
                    kind: "Отдел".to_owned(),
                    parent_id: None,
                    is_department: false,
                    is_portal: false,
                    x: 100.0,
                    y: 500.0,
                    inputs: vec![Metric {
                        id: "M_SALES_LEADS".to_owned(),
                        name: "Лиды".to_owned(),
                        value: 21_427.0,
                        required: 18_000.0,
                        max: None,
                        unit: "ед.".to_owned(),
                    }],
                    outputs: vec![Metric {
                        id: "M_REVENUE".to_owned(),
                        name: "Выручка".to_owned(),
                        value: 51_642.0,
                        required: 45_000.0,
                        max: Some(75_000.0),
                        unit: "$".to_owned(),
                    }],
                    poll_url: String::new(),
                    poll_interval_secs: 5,
                },
            ],
            edges: vec![Edge {
                from_node: "PROC_MKT_01".to_owned(),
                from_metric: "M_LEADS".to_owned(),
                to_node: "PROC_SALES_01".to_owned(),
                to_metric: "M_SALES_LEADS".to_owned(),
            }],
        }
    }
}

impl Project {
    pub fn to_pg(&self) -> Result<String, serde_json::Error> {
        let body = serde_json::to_string_pretty(self)?;
        Ok(format!("polygram/1\n{body}\n"))
    }

    pub fn from_pg(contents: &str) -> Result<Self, String> {
        let json = contents
            .strip_prefix("polygram/1")
            .ok_or_else(|| "Неподдерживаемый формат .pg (ожидается polygram/1)".to_owned())?
            .trim();
        let project: Self =
            serde_json::from_str(json).map_err(|error| format!("Ошибка разбора .pg: {error}"))?;
        if project.storage_id.trim().is_empty() {
            return Err("Пустой ID проекта для локального хранилища".to_owned());
        }
        project.validate()?;
        Ok(project)
    }

    pub fn validate(&self) -> Result<(), String> {
        let mut node_ids = HashSet::new();
        let mut metric_ids = HashSet::new();
        for node in &self.nodes {
            if node.is_department && node.is_portal {
                return Err(format!(
                    "Узел не может быть одновременно отделом и порталом: {}",
                    node.id
                ));
            }
            if node.id.trim().is_empty() || !node_ids.insert(node.id.as_str()) {
                return Err(format!("Пустой или повторяющийся ID узла: {}", node.id));
            }
            if let Some(parent_id) = &node.parent_id
                && (parent_id == &node.id
                    || !self
                        .nodes
                        .iter()
                        .any(|parent| parent.id == *parent_id && parent.is_department))
            {
                return Err(format!("Некорректный отдел для узла: {}", node.id));
            }
            for metric in node.inputs.iter().chain(node.outputs.iter()) {
                if metric.id.trim().is_empty() || !metric_ids.insert(metric.id.as_str()) {
                    return Err(format!(
                        "Пустой или повторяющийся ID метрики: {}",
                        metric.id
                    ));
                }
                for node in &self.nodes {
                    let mut parent_id = node.parent_id.as_deref();
                    let mut ancestors = HashSet::new();
                    while let Some(id) = parent_id {
                        if !ancestors.insert(id) {
                            return Err(format!(
                                "Циклическая иерархия отделов для узла: {}",
                                node.id
                            ));
                        }
                        parent_id = self.node(id).and_then(|parent| parent.parent_id.as_deref());
                    }
                }
                if !metric.value.is_finite()
                    || !metric.required.is_finite()
                    || metric.required < 0.0
                    || metric.max.is_some_and(|max| !max.is_finite() || max < 0.0)
                {
                    return Err(format!("Некорректные значения показателя: {}", metric.id));
                }
            }
        }
        let mut edge_ids = HashSet::new();
        for edge in &self.edges {
            let Some(from_node) = self.node(&edge.from_node) else {
                return Err("Связь указывает на отсутствующий узел".to_owned());
            };
            let Some(to_node) = self.node(&edge.to_node) else {
                return Err("Связь указывает на отсутствующий узел".to_owned());
            };
            if edge.from_node == edge.to_node {
                return Err("Связь не может вести в тот же блок".to_owned());
            }
            if from_node.parent_id != to_node.parent_id
                && !from_node.is_portal
                && !to_node.is_portal
            {
                return Err("Между отделами связь должна проходить через портал".to_owned());
            }
            if !self.metric_exists(&edge.from_node, &edge.from_metric, false)
                || !self.metric_exists(&edge.to_node, &edge.to_metric, true)
            {
                return Err("Связь указывает на отсутствующую метрику".to_owned());
            }
            if !edge_ids.insert((
                edge.from_node.as_str(),
                edge.from_metric.as_str(),
                edge.to_node.as_str(),
                edge.to_metric.as_str(),
            )) {
                return Err("Повторяющаяся связь между показателями".to_owned());
            }
        }
        Ok(())
    }

    pub fn node(&self, id: &str) -> Option<&Node> {
        self.nodes.iter().find(|node| node.id == id)
    }

    pub fn node_mut(&mut self, id: &str) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|node| node.id == id)
    }

    pub fn metric_exists(&self, node_id: &str, metric_id: &str, input: bool) -> bool {
        self.node(node_id).is_some_and(|node| {
            let metrics = if input { &node.inputs } else { &node.outputs };
            metrics.iter().any(|metric| metric.id == metric_id)
        })
    }

    pub fn layout_left_to_right_in_scope(&mut self, parent_id: Option<&str>) {
        let visible_ids: Vec<_> = self
            .nodes
            .iter()
            .filter(|node| node.parent_id.as_deref() == parent_id)
            .map(|node| node.id.clone())
            .collect();
        if visible_ids.is_empty() {
            return;
        }
        let mut graph = DiGraph::new();
        let mut indices = HashMap::new();
        for id in &visible_ids {
            let index = graph.add_node(id.clone());
            indices.insert(id.as_str(), index);
        }
        for edge in &self.edges {
            if let (Some(from), Some(to)) = (
                indices.get(edge.from_node.as_str()),
                indices.get(edge.to_node.as_str()),
            ) {
                graph.add_edge(*from, *to, ());
            }
        }
        let mut rank = HashMap::new();
        if let Ok(order) = petgraph::algo::toposort(&graph, None) {
            for index in order {
                let depth = graph
                    .neighbors_directed(index, petgraph::Direction::Incoming)
                    .filter_map(|parent| rank.get(&parent).copied())
                    .max()
                    .map_or(0, |parent_rank| parent_rank + 1);
                rank.insert(index, depth);
            }
        }
        let mut columns: BTreeMap<usize, Vec<String>> = BTreeMap::new();
        for index in graph.node_indices() {
            let node_id = &graph[index];
            columns
                .entry(rank.get(&index).copied().unwrap_or(0))
                .or_default()
                .push(node_id.clone());
        }
        for (column, ids) in columns {
            for (row, id) in ids.iter().enumerate() {
                if let Some(node) = self.node_mut(id) {
                    node.x = 100.0 + column as f32 * 440.0;
                    node.y = 100.0 + row as f32 * 330.0;
                }
            }
        }
    }

    pub fn apply_tick(&mut self, node_id: &str, metric_id: &str, value: f64) -> bool {
        if !value.is_finite() {
            return false;
        }
        let Some(node) = self.node_mut(node_id) else {
            return false;
        };
        let metric = node
            .inputs
            .iter_mut()
            .chain(node.outputs.iter_mut())
            .find(|metric| metric.id == metric_id);
        if let Some(metric) = metric {
            metric.value = value;
            true
        } else {
            false
        }
    }
}

pub const NODE_WIDTH: f32 = 340.0;
pub const NODE_MIN_HEIGHT: f32 = 260.0;

pub fn node_height(node: &Node) -> f32 {
    if node.is_department {
        return 132.0;
    }
    NODE_MIN_HEIGHT
        + node.inputs.len().saturating_sub(1) as f32 * 38.0
        + node.outputs.len().saturating_sub(1) as f32 * 38.0
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpatialEntry {
    pub id: String,
    pub envelope: AABB<[f64; 2]>,
}

impl SpatialEntry {
    pub fn for_node(node: &Node) -> Self {
        Self {
            id: node.id.clone(),
            envelope: AABB::from_corners(
                [node.x as f64, node.y as f64],
                [
                    (node.x + NODE_WIDTH) as f64,
                    (node.y + node_height(node)) as f64,
                ],
            ),
        }
    }
}

impl RTreeObject for SpatialEntry {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        self.envelope
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_project_round_trips_through_pg_format() {
        let project = Project::default();
        let serialized = project.to_pg().expect("project serializes");
        let parsed = Project::from_pg(&serialized).expect("project parses");
        assert_eq!(parsed.name, project.name);
        assert_eq!(parsed.storage_id, project.storage_id);
        assert_eq!(parsed.nodes.len(), 2);
        assert_eq!(parsed.edges.len(), 1);
    }

    #[test]
    fn rejects_dangling_edges() {
        let mut project = Project::default();
        project.edges[0].to_node = "MISSING_NODE".to_owned();
        assert!(project.validate().is_err());
    }

    #[test]
    fn applies_metric_updates_by_node_and_metric_id() {
        let mut project = Project::default();
        assert!(project.apply_tick("PROC_MKT_01", "M_LEADS", 450.5));
        assert_eq!(project.node("PROC_MKT_01").unwrap().outputs[0].value, 450.5);
        assert!(!project.apply_tick("unknown", "M_LEADS", 10.0));
        assert!(!project.apply_tick("PROC_MKT_01", "M_LEADS", f64::NAN));
    }

    #[test]
    fn rejects_self_links_and_duplicate_metric_connections() {
        let mut project = Project::default();
        let edge = project.edges[0].clone();
        project.edges.push(edge.clone());
        assert!(project.validate().is_err());

        project.edges.pop();
        project.edges[0].to_node = project.edges[0].from_node.clone();
        assert!(project.validate().is_err());
    }

    #[test]
    fn rejects_non_finite_or_negative_metric_targets() {
        let mut project = Project::default();
        project.nodes[0].outputs[0].required = f64::NAN;
        assert!(project.validate().is_err());

        project.nodes[0].outputs[0].required = -1.0;
        assert!(project.validate().is_err());
    }

    #[test]
    fn validates_department_hierarchy_and_rejects_cycles() {
        let mut project = Project::default();
        project.edges.clear();
        project.nodes[0].is_department = true;
        project.nodes[1].parent_id = Some(project.nodes[0].id.clone());
        assert!(project.validate().is_ok());

        project.nodes[1].is_department = true;
        project.nodes[0].parent_id = Some(project.nodes[1].id.clone());
        assert!(project.validate().is_err());
    }

    #[test]
    fn cross_department_links_require_a_portal_endpoint() {
        let mut project = Project::default();
        project.nodes[0].is_portal = true;
        project.nodes[1].parent_id = Some("DEPT_01".to_owned());
        let mut department = project.nodes[0].clone();
        department.id = "DEPT_01".to_owned();
        department.is_department = true;
        department.is_portal = false;
        department.inputs.clear();
        department.outputs.clear();
        project.nodes.push(department);

        assert!(project.validate().is_ok());
        let serialized = project.to_pg().expect("project serializes");
        let parsed = Project::from_pg(&serialized).expect("hierarchy loads again");
        assert!(parsed.node("PROC_MKT_01").unwrap().is_portal);
        assert_eq!(
            parsed.node("PROC_SALES_01").unwrap().parent_id.as_deref(),
            Some("DEPT_01")
        );

        project.nodes[0].is_portal = false;
        assert!(project.validate().is_err());
    }

    #[test]
    fn left_to_right_layout_keeps_nested_nodes_in_their_own_scope() {
        let mut project = Project::default();
        project.nodes[0].is_department = true;
        project.nodes[1].parent_id = Some(project.nodes[0].id.clone());
        project.nodes[1].x = 1_200.0;
        project.nodes[1].y = 850.0;

        project.layout_left_to_right_in_scope(None);

        assert_eq!(project.nodes[0].x, 100.0);
        assert_eq!(project.nodes[1].x, 1_200.0);
        assert_eq!(project.nodes[1].y, 850.0);
    }
}

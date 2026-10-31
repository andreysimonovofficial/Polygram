use crate::{
    extensions::{
        ExtensionBlock, ExtensionFailure, ExtensionScan, LoadedExtension, extension_roots,
        scan_extensions,
    },
    local_store::LocalStore,
    model::{Edge, Metric, NODE_WIDTH, Node, Project, SpatialEntry, fresh_storage_id, node_height},
    network::{Network, PollConfig, Tick},
};
use eframe::egui::{
    self, Align2, Color32, FontId, Key, PointerButton, Pos2, Rect, Sense, Stroke, Vec2,
};
use rstar::RTree;
use std::{
    collections::{HashMap, VecDeque},
    fs,
    path::PathBuf,
    time::Instant,
};

const BG: Color32 = Color32::from_rgb(8, 8, 10);
const CARD: Color32 = Color32::from_rgba_premultiplied(17, 18, 24, 214);
const PANEL: Color32 = Color32::from_rgba_premultiplied(11, 12, 16, 199);
const INSET: Color32 = Color32::from_rgb(19, 20, 25);
const HOVER: Color32 = Color32::from_rgb(27, 28, 33);
const BORDER: Color32 = Color32::from_rgb(34, 35, 40);
const BORDER_ACTIVE: Color32 = Color32::from_rgb(58, 59, 65);
const TEXT: Color32 = Color32::from_rgb(237, 237, 240);
const MUTED: Color32 = Color32::from_rgb(142, 142, 152);
const BLUE: Color32 = Color32::from_rgb(59, 130, 246);
const GREEN: Color32 = Color32::from_rgb(16, 185, 129);
const AMBER: Color32 = Color32::from_rgb(198, 161, 91);
const RED: Color32 = Color32::from_rgb(244, 63, 94);
const PURPLE: Color32 = Color32::from_rgb(168, 85, 247);
const DOCK_ICON_COUNT: f32 = 9.0;

fn load_embedded_texture(ctx: &egui::Context, name: &str, bytes: &[u8]) -> egui::TextureHandle {
    let decoded = image::load_from_memory(bytes).expect("embedded reference PNG must be valid");
    let rgba = decoded.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
    ctx.load_texture(name, image, egui::TextureOptions::LINEAR)
}

fn paint_dock_icon(
    painter: &egui::Painter,
    texture: &egui::TextureHandle,
    index: usize,
    rect: Rect,
    tint: Color32,
) {
    let left = index as f32 / DOCK_ICON_COUNT;
    let right = (index + 1) as f32 / DOCK_ICON_COUNT;
    let uv = Rect::from_min_max(Pos2::new(left, 0.0), Pos2::new(right, 1.0));
    painter.image(texture.id(), rect, uv, tint);
}

fn dock_icon_button(
    ui: &mut egui::Ui,
    texture: &egui::TextureHandle,
    index: usize,
    active: bool,
) -> egui::Response {
    let response = ui.add_sized([28.0, 28.0], egui::Button::new("").selected(active));
    paint_dock_icon(
        ui.painter(),
        texture,
        index,
        response.rect.shrink(6.0),
        if active || response.hovered() {
            TEXT
        } else {
            MUTED
        },
    );
    response
}

fn apply_visual_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    let visuals = &mut style.visuals;
    *visuals = egui::Visuals::dark();
    visuals.dark_mode = true;
    visuals.panel_fill = PANEL;
    visuals.window_fill = CARD;
    visuals.extreme_bg_color = BG;
    visuals.faint_bg_color = INSET;
    visuals.code_bg_color = INSET;
    visuals.hyperlink_color = MUTED;
    visuals.warn_fg_color = MUTED;
    visuals.error_fg_color = MUTED;
    visuals.selection.bg_fill = HOVER;
    visuals.selection.stroke = Stroke::new(1.0_f32, BORDER_ACTIVE);
    visuals.widgets.noninteractive.bg_fill = PANEL;
    visuals.widgets.noninteractive.weak_bg_fill = PANEL;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT);
    visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
    visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    visuals.widgets.inactive.bg_stroke = Stroke::NONE;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, MUTED);
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(10);
    visuals.widgets.hovered.bg_fill = HOVER;
    visuals.widgets.hovered.weak_bg_fill = HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::NONE;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, TEXT);
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(10);
    visuals.widgets.active.bg_fill = HOVER;
    visuals.widgets.active.weak_bg_fill = HOVER;
    visuals.widgets.active.bg_stroke = Stroke::NONE;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, TEXT);
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(10);
    style.spacing.item_spacing = Vec2::new(8.0, 7.0);
    style.spacing.button_padding = Vec2::new(10.0, 6.0);
    style
        .text_styles
        .insert(egui::TextStyle::Body, FontId::proportional(12.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, FontId::proportional(12.0));
    style
        .text_styles
        .insert(egui::TextStyle::Small, FontId::proportional(10.0));
    style
        .text_styles
        .insert(egui::TextStyle::Heading, FontId::proportional(15.0));
    ctx.set_style(style);
}

#[derive(Clone, Copy)]
struct ResolvedEdge {
    from_node: usize,
    from_metric: usize,
    to_node: usize,
    to_metric: usize,
}

#[derive(Clone, Copy, Default)]
struct FrameSample {
    frame_ms: f32,
    update_ms: f32,
}

#[derive(Clone, Copy, Default)]
struct StressSample {
    update_ms: f32,
    changed_metrics: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CanvasTool {
    Select,
    Pan,
    Connect,
}

#[derive(Clone, Copy)]
enum MetricDirection {
    Input,
    Output,
}

#[derive(Clone)]
struct MetricDraft {
    node_id: String,
    direction: MetricDirection,
    name: String,
    value: String,
    required: String,
    unit: String,
    max: String,
    error: Option<String>,
}

pub struct PolygramApp {
    project: Project,
    spatial: RTree<SpatialEntry>,
    node_indices: HashMap<String, usize>,
    resolved_edges: Vec<ResolvedEdge>,
    edge_allocations: Vec<f64>,
    input_supply: Vec<Vec<f64>>,
    input_has_connections: Vec<Vec<bool>>,
    output_demand: Vec<Vec<f64>>,
    output_has_connections: Vec<Vec<bool>>,
    output_limits: Vec<Vec<Option<f64>>>,
    renderer: String,
    network: Network,
    local_store: LocalStore,
    pending_store_ticks: VecDeque<(String, Tick)>,
    next_store_retry: Instant,
    selected: Option<String>,
    selected_edge: Option<usize>,
    connecting_from: Option<(String, String)>,
    dragging_node: Option<String>,
    active_tool: CanvasTool,
    context_menu_world: Option<Pos2>,
    context_menu_node: Option<String>,
    add_process_requested: bool,
    add_department_requested: bool,
    add_portal_requested: bool,
    current_scope: Option<String>,
    search_open: bool,
    search_query: String,
    focus_node_requested: Option<String>,
    metric_draft: Option<MetricDraft>,
    zoom: f32,
    zoom_target: f32,
    zoom_anchor: Option<(Vec2, Pos2)>,
    zoom_step_requested: f32,
    zoom_reset_requested: bool,
    camera_offset: Vec2,
    notice: String,
    applied_ticks: u64,
    dropped_ticks: u64,
    frame_samples: VecDeque<FrameSample>,
    next_node_number: u64,
    animation_enabled: bool,
    frame_cap: u32,
    stress_running: bool,
    stress_speed: f32,
    stress_elapsed: f64,
    stress_samples: VecDeque<StressSample>,
    stress_ticks: u64,
    qa_open: bool,
    fit_graph_requested: bool,
    reference_logo: Option<egui::TextureHandle>,
    reference_icons: Option<egui::TextureHandle>,
    visible_nodes: usize,
    visible_edges: usize,
    last_frame_at: Option<Instant>,
    window_focused: bool,
    theme_applied: bool,
    extension_roots: Vec<PathBuf>,
    extensions: Vec<LoadedExtension>,
    extension_failures: Vec<ExtensionFailure>,
    next_extension_scan: Instant,
}

impl PolygramApp {
    pub fn new(
        network: Network,
        renderer: String,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let local_store = LocalStore::open_default()?;
        let mut project = Project::default();
        for value in local_store.load_latest(&project)? {
            project.apply_tick(&value.node_id, &value.metric_id, value.value);
        }
        let spatial = RTree::bulk_load(project.nodes.iter().map(SpatialEntry::for_node).collect());
        let mut app = Self {
            project,
            spatial,
            node_indices: HashMap::new(),
            resolved_edges: Vec::new(),
            edge_allocations: Vec::new(),
            input_supply: Vec::new(),
            input_has_connections: Vec::new(),
            output_demand: Vec::new(),
            output_has_connections: Vec::new(),
            output_limits: Vec::new(),
            renderer,
            network,
            local_store,
            pending_store_ticks: VecDeque::new(),
            next_store_retry: Instant::now(),
            selected: None,
            selected_edge: None,
            connecting_from: None,
            dragging_node: None,
            active_tool: CanvasTool::Select,
            context_menu_world: None,
            context_menu_node: None,
            add_process_requested: false,
            add_department_requested: false,
            add_portal_requested: false,
            current_scope: None,
            search_open: false,
            search_query: String::new(),
            focus_node_requested: None,
            metric_draft: None,
            zoom: 1.0,
            zoom_target: 1.0,
            zoom_anchor: None,
            zoom_step_requested: 0.0,
            zoom_reset_requested: false,
            camera_offset: Vec2::ZERO,
            notice: "Локальный webhook: http://localhost:8080/api/v1/tick".to_owned(),
            applied_ticks: 0,
            dropped_ticks: 0,
            frame_samples: VecDeque::with_capacity(300),
            next_node_number: 1,
            animation_enabled: true,
            frame_cap: 144,
            stress_running: false,
            stress_speed: 10.0,
            stress_elapsed: 0.0,
            stress_samples: VecDeque::with_capacity(300),
            stress_ticks: 0,
            qa_open: false,
            fit_graph_requested: false,
            reference_logo: None,
            reference_icons: None,
            visible_nodes: 0,
            visible_edges: 0,
            last_frame_at: None,
            window_focused: false,
            theme_applied: false,
            extension_roots: extension_roots(),
            extensions: Vec::new(),
            extension_failures: Vec::new(),
            next_extension_scan: Instant::now(),
        };
        app.rebuild_render_indexes();
        app.refresh_extensions();
        Ok(app)
    }

    fn rebuild_spatial(&mut self) {
        self.spatial = RTree::bulk_load(
            self.project
                .nodes
                .iter()
                .map(SpatialEntry::for_node)
                .collect(),
        );
        self.rebuild_render_indexes();
    }

    fn rebuild_render_indexes(&mut self) {
        self.node_indices.clear();
        self.node_indices.reserve(self.project.nodes.len());
        for (index, node) in self.project.nodes.iter().enumerate() {
            self.node_indices.insert(node.id.clone(), index);
        }

        self.resolved_edges.clear();
        self.resolved_edges.reserve(self.project.edges.len());
        self.output_limits = self
            .project
            .nodes
            .iter()
            .map(|node| node.outputs.iter().map(|metric| metric.max).collect())
            .collect();
        self.input_supply = self
            .project
            .nodes
            .iter()
            .map(|node| vec![0.0; node.inputs.len()])
            .collect();
        self.input_has_connections = self
            .project
            .nodes
            .iter()
            .map(|node| vec![false; node.inputs.len()])
            .collect();
        self.output_demand = self
            .project
            .nodes
            .iter()
            .map(|node| vec![0.0; node.outputs.len()])
            .collect();
        self.output_has_connections = self
            .project
            .nodes
            .iter()
            .map(|node| vec![false; node.outputs.len()])
            .collect();
        let mut target_limit_sums: Vec<Vec<f64>> = self
            .project
            .nodes
            .iter()
            .map(|node| vec![0.0; node.outputs.len()])
            .collect();
        let mut target_counts: Vec<Vec<usize>> = self
            .project
            .nodes
            .iter()
            .map(|node| vec![0; node.outputs.len()])
            .collect();
        let mut all_targets_have_limits: Vec<Vec<bool>> = self
            .project
            .nodes
            .iter()
            .map(|node| vec![true; node.outputs.len()])
            .collect();

        for edge in &self.project.edges {
            let (Some(&from_node), Some(&to_node)) = (
                self.node_indices.get(&edge.from_node),
                self.node_indices.get(&edge.to_node),
            ) else {
                continue;
            };
            let Some(from_metric) = self.project.nodes[from_node]
                .outputs
                .iter()
                .position(|metric| metric.id == edge.from_metric)
            else {
                continue;
            };
            let Some(to_metric) = self.project.nodes[to_node]
                .inputs
                .iter()
                .position(|metric| metric.id == edge.to_metric)
            else {
                continue;
            };
            self.resolved_edges.push(ResolvedEdge {
                from_node,
                from_metric,
                to_node,
                to_metric,
            });
            target_counts[from_node][from_metric] += 1;
            if let Some(max) = self.project.nodes[to_node].inputs[to_metric].max {
                target_limit_sums[from_node][from_metric] += max;
            } else {
                all_targets_have_limits[from_node][from_metric] = false;
            }
        }
        for (node_index, node) in self.project.nodes.iter().enumerate() {
            for (metric_index, metric) in node.outputs.iter().enumerate() {
                self.output_limits[node_index][metric_index] = effective_output_limit(
                    metric.max,
                    target_counts[node_index][metric_index],
                    all_targets_have_limits[node_index][metric_index],
                    target_limit_sums[node_index][metric_index],
                );
            }
        }
        self.edge_allocations = vec![0.0; self.resolved_edges.len()];
        self.refresh_edge_flows();
    }

    fn refresh_edge_flows(&mut self) {
        let passes = if self.project.nodes.iter().any(|node| node.is_portal) {
            2
        } else {
            1
        };
        for pass in 0..passes {
            self.edge_allocations.fill(0.0);
            for node in &mut self.input_supply {
                node.fill(0.0);
            }
            for node in &mut self.input_has_connections {
                node.fill(false);
            }
            for node in &mut self.output_demand {
                node.fill(0.0);
            }
            for node in &mut self.output_has_connections {
                node.fill(false);
            }

            for edge in &self.resolved_edges {
                let target = &self.project.nodes[edge.to_node].inputs[edge.to_metric];
                self.output_demand[edge.from_node][edge.from_metric] += target.required.max(0.0);
                self.output_has_connections[edge.from_node][edge.from_metric] = true;
                self.input_has_connections[edge.to_node][edge.to_metric] = true;
            }

            for (edge_index, edge) in self.resolved_edges.iter().enumerate() {
                let source = &self.project.nodes[edge.from_node].outputs[edge.from_metric];
                let target = &self.project.nodes[edge.to_node].inputs[edge.to_metric];
                let total_demand = self.output_demand[edge.from_node][edge.from_metric];
                self.edge_allocations[edge_index] = if total_demand <= 0.0 {
                    0.0
                } else {
                    target.required.max(0.0) * (source.value.max(0.0) / total_demand).min(1.0)
                };
            }

            for (edge_index, edge) in self.resolved_edges.iter().enumerate() {
                self.input_supply[edge.to_node][edge.to_metric] +=
                    self.edge_allocations[edge_index];
            }

            if pass == 0 && passes == 2 {
                for (node_index, node) in self.project.nodes.iter_mut().enumerate() {
                    if !node.is_portal {
                        continue;
                    }
                    for metric_index in 0..node.outputs.len().min(node.inputs.len()) {
                        node.outputs[metric_index].value =
                            self.input_supply[node_index][metric_index];
                    }
                }
            }
        }
    }

    fn sync_polling(&self) {
        let configs: Vec<_> = self
            .project
            .nodes
            .iter()
            .filter(|node| !node.poll_url.trim().is_empty())
            .map(|node| PollConfig {
                node_id: node.id.clone(),
                url: node.poll_url.clone(),
                interval_secs: node.poll_interval_secs.max(1),
            })
            .collect();
        let unchanged = match self.network.polls.read() {
            Ok(current) => *current == configs,
            Err(error) => {
                self.notice_with_error(format!("Ошибка чтения настройки polling: {error}"));
                return;
            }
        };
        if !unchanged {
            match self.network.polls.write() {
                Ok(mut current) => *current = configs,
                Err(error) => {
                    self.notice_with_error(format!("Ошибка настройки polling: {error}"));
                }
            }
        }
    }

    fn notice_with_error(&self, message: String) {
        eprintln!("{message}");
    }

    fn load_project(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Polygram graph", &["pg"])
            .pick_file()
        else {
            return;
        };
        let result = fs::read_to_string(&path)
            .map_err(|error| error.to_string())
            .and_then(|contents| Project::from_pg(&contents));
        match result {
            Ok(project) => {
                self.project = project;
                match self.local_store.load_latest(&self.project) {
                    Ok(values) => {
                        for value in values {
                            self.project
                                .apply_tick(&value.node_id, &value.metric_id, value.value);
                        }
                    }
                    Err(error) => {
                        self.notice = format!(
                            "Проект открыт, но не удалось восстановить сохранённые значения: {error}"
                        );
                    }
                }
                self.stress_running = false;
                self.stress_samples.clear();
                self.stress_ticks = 0;
                self.stress_elapsed = 0.0;
                self.current_scope = None;
                self.selected = None;
                self.selected_edge = None;
                self.connecting_from = None;
                self.rebuild_spatial();
                self.sync_polling();
                self.notice = format!("Открыт {}", path.display());
            }
            Err(error) => self.notice = format!("Не удалось открыть проект: {error}"),
        }
    }

    fn save_project(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Polygram graph", &["pg"])
            .set_file_name("project.pg")
            .save_file()
        else {
            return;
        };
        let result = self
            .project
            .to_pg()
            .map_err(|error| error.to_string())
            .and_then(|contents| fs::write(&path, contents).map_err(|error| error.to_string()));
        self.notice = match result {
            Ok(()) => format!("Сохранено {}", path.display()),
            Err(error) => format!("Не удалось сохранить проект: {error}"),
        };
    }

    fn add_node(&mut self, world: Pos2) {
        let id = loop {
            self.next_node_number += 1;
            let candidate = format!("PROC_{:04}", self.next_node_number);
            if self.project.node(&candidate).is_none() {
                break candidate;
            }
        };
        let node = Node {
            id: id.clone(),
            name: format!("Новый процесс {}", self.next_node_number),
            kind: "Процесс".to_owned(),
            parent_id: self.current_scope.clone(),
            is_department: false,
            is_portal: false,
            x: world.x,
            y: world.y,
            inputs: vec![Metric {
                id: format!("{}_IN_01", id),
                name: "Новый запрос".to_owned(),
                value: 0.0,
                required: 100.0,
                max: None,
                unit: "ед.".to_owned(),
            }],
            outputs: vec![Metric {
                id: format!("{}_OUT_01", id),
                name: "Новый выход".to_owned(),
                value: 0.0,
                required: 100.0,
                max: Some(500.0),
                unit: "ед.".to_owned(),
            }],
            poll_url: String::new(),
            poll_interval_secs: 5,
        };
        self.project.nodes.push(node);
        self.rebuild_spatial();
        self.selected = Some(id);
        self.selected_edge = None;
    }

    fn add_department(&mut self, world: Pos2) {
        let id = self.next_node_id("DEPT");
        let node = Node {
            id: id.clone(),
            name: format!("Новый отдел {}", self.next_node_number),
            kind: "Отдел".to_owned(),
            parent_id: self.current_scope.clone(),
            is_department: true,
            is_portal: false,
            x: world.x,
            y: world.y,
            inputs: Vec::new(),
            outputs: Vec::new(),
            poll_url: String::new(),
            poll_interval_secs: 5,
        };
        self.project.nodes.push(node);
        self.rebuild_spatial();
        self.selected = Some(id);
        self.selected_edge = None;
    }

    fn add_portal(&mut self, world: Pos2) {
        let id = self.next_node_id("PORTAL");
        let node = Node {
            id: id.clone(),
            name: format!("Портал {}", self.next_node_number),
            kind: "Портал".to_owned(),
            parent_id: self.current_scope.clone(),
            is_department: false,
            is_portal: true,
            x: world.x,
            y: world.y,
            inputs: vec![Metric {
                id: format!("{id}_IN"),
                name: "Входящий поток".to_owned(),
                value: 0.0,
                required: 100.0,
                max: None,
                unit: "ед.".to_owned(),
            }],
            outputs: vec![Metric {
                id: format!("{id}_OUT"),
                name: "Исходящий поток".to_owned(),
                value: 0.0,
                required: 100.0,
                max: None,
                unit: "ед.".to_owned(),
            }],
            poll_url: String::new(),
            poll_interval_secs: 5,
        };
        self.project.nodes.push(node);
        self.rebuild_spatial();
        self.selected = Some(id);
        self.selected_edge = None;
    }

    fn next_node_id(&mut self, prefix: &str) -> String {
        loop {
            self.next_node_number += 1;
            let candidate = format!("{prefix}_{:04}", self.next_node_number);
            if self.project.node(&candidate).is_none() {
                return candidate;
            }
        }
    }

    fn refresh_extensions(&mut self) {
        let ExtensionScan {
            extensions,
            failures,
        } = scan_extensions(&self.extension_roots);
        self.extensions = extensions;
        self.extension_failures = failures;
        self.next_extension_scan = Instant::now() + std::time::Duration::from_secs(1);
    }

    fn add_extension_block(&mut self, extension_id: &str, block: &ExtensionBlock) {
        let Some(extension_name) = self
            .extensions
            .iter()
            .find(|extension| extension.manifest.id == extension_id)
            .map(|extension| extension.manifest.name.clone())
        else {
            self.notice = "Расширение уже недоступно; обновите список расширений".to_owned();
            return;
        };
        let id = self.next_node_id("EXT");
        let node = Node {
            id: id.clone(),
            name: block.name.clone(),
            kind: block.kind.clone(),
            parent_id: self.current_scope.clone(),
            is_department: false,
            is_portal: false,
            x: 180.0 + self.project.nodes.len() as f32 * 24.0,
            y: 180.0 + self.project.nodes.len() as f32 * 24.0,
            inputs: block
                .inputs
                .iter()
                .enumerate()
                .map(|(index, metric)| self.extension_metric(&id, "IN", index, metric))
                .collect(),
            outputs: block
                .outputs
                .iter()
                .enumerate()
                .map(|(index, metric)| self.extension_metric(&id, "OUT", index, metric))
                .collect(),
            poll_url: String::new(),
            poll_interval_secs: 5,
        };
        self.project.nodes.push(node);
        self.rebuild_spatial();
        self.selected = Some(id);
        self.selected_edge = None;
        self.fit_graph_requested = true;
        self.notice = format!(
            "Добавлен блок «{}» из расширения «{}»",
            block.name, extension_name
        );
    }

    fn extension_metric(
        &self,
        node_id: &str,
        direction: &str,
        index: usize,
        source: &crate::extensions::ExtensionMetric,
    ) -> Metric {
        Metric {
            id: format!("{node_id}_{direction}_{:02}", index + 1),
            name: source.name.clone(),
            value: source.value,
            required: source.required,
            max: source.max,
            unit: source.unit.clone(),
        }
    }

    fn enter_department(&mut self, department_id: &str) {
        if self
            .project
            .node(department_id)
            .is_some_and(|node| node.is_department)
        {
            self.current_scope = Some(department_id.to_owned());
            self.selected = None;
            self.selected_edge = None;
            self.connecting_from = None;
            self.fit_graph_requested = true;
        }
    }

    fn go_to_parent_scope(&mut self) {
        self.current_scope = self
            .current_scope
            .as_deref()
            .and_then(|id| self.project.node(id))
            .and_then(|node| node.parent_id.clone());
        self.selected = None;
        self.selected_edge = None;
        self.connecting_from = None;
        self.fit_graph_requested = true;
    }

    fn generate_sample(&mut self, count: usize) {
        self.stress_running = false;
        self.stress_samples.clear();
        self.stress_ticks = 0;
        self.stress_elapsed = 0.0;
        let mut nodes = Vec::with_capacity(count);
        let mut edges = Vec::with_capacity(count.saturating_sub(1));
        for index in 0..count {
            let id = format!("PROC_{:04}", index + 1);
            let input_id = format!("M_{:04}_IN", index + 1);
            let output_id = format!("M_{:04}_OUT", index + 1);
            nodes.push(Node {
                id: id.clone(),
                name: format!("Процесс {}", index + 1),
                kind: "Процесс".to_owned(),
                parent_id: None,
                is_department: false,
                is_portal: false,
                x: 100.0 + (index % 10) as f32 * 440.0,
                y: 100.0 + (index / 10) as f32 * 330.0,
                inputs: vec![Metric {
                    id: input_id.clone(),
                    name: "Входящий поток".to_owned(),
                    value: 180.0 + (index % 280) as f64,
                    required: 180.0 + (index % 280) as f64,
                    max: None,
                    unit: "ед.".to_owned(),
                }],
                outputs: vec![Metric {
                    id: output_id.clone(),
                    name: "Исходящий поток".to_owned(),
                    value: 180.0 + (index * 37 % 280) as f64,
                    required: 180.0 + (index * 37 % 280) as f64,
                    max: Some((180.0 + (index * 37 % 280) as f64) * 1.6),
                    unit: "ед.".to_owned(),
                }],
                poll_url: String::new(),
                poll_interval_secs: 5,
            });
            if index > 0 {
                edges.push(Edge {
                    from_node: format!("PROC_{:04}", index),
                    from_metric: format!("M_{:04}_OUT", index),
                    to_node: id.clone(),
                    to_metric: input_id.clone(),
                });
            }
            if index > 1 {
                edges.push(Edge {
                    from_node: format!("PROC_{:04}", index - 1),
                    from_metric: format!("M_{:04}_OUT", index - 1),
                    to_node: id.clone(),
                    to_metric: input_id.clone(),
                });
            }
            if (3..=5).contains(&index) {
                edges.push(Edge {
                    from_node: format!("PROC_{:04}", index - 2),
                    from_metric: format!("M_{:04}_OUT", index - 2),
                    to_node: id.clone(),
                    to_metric: input_id,
                });
            }
        }
        self.project = Project {
            storage_id: fresh_storage_id(),
            name: format!("Тестовый граф ({count} узлов)"),
            nodes,
            edges,
        };
        self.current_scope = None;
        self.selected = None;
        self.selected_edge = None;
        self.connecting_from = None;
        self.rebuild_spatial();
        self.sync_polling();
        self.notice = format!("Сгенерировано узлов: {count}");
    }

    fn start_stress_test(&mut self) {
        self.generate_sample(1_000);
        self.stress_speed = 10.0;
        self.stress_elapsed = 0.0;
        self.stress_ticks = 0;
        self.stress_samples.clear();
        self.stress_running = true;
        self.fit_graph_requested = true;
        self.notice = "Стресс-тест: 1 000 узлов, 2 000 связей, динамические метрики 10×".to_owned();
    }

    fn update_stress_test(&mut self, dt: f32) {
        if !self.stress_running {
            return;
        }
        let started = Instant::now();
        self.stress_elapsed += dt.max(0.0) as f64 * self.stress_speed as f64;
        let changed_metrics = update_dynamic_metrics(&mut self.project, self.stress_elapsed);
        self.refresh_edge_flows();
        self.stress_ticks += 1;
        if self.stress_samples.len() == 300 {
            self.stress_samples.pop_front();
        }
        self.stress_samples.push_back(StressSample {
            update_ms: started.elapsed().as_secs_f32() * 1000.0,
            changed_metrics,
        });
    }

    fn apply_pending_ticks(&mut self) {
        let mut handled = 0;
        while handled < 4096 && self.pending_store_ticks.len() < 65_536 {
            let Some(tick) = self.network.ticks.pop() else {
                break;
            };
            if self
                .project
                .apply_tick(&tick.node_id, &tick.metric_id, tick.value)
            {
                self.applied_ticks += 1;
                self.pending_store_ticks
                    .push_back((self.project.storage_id.clone(), tick));
            } else {
                self.dropped_ticks += 1;
            }
            handled += 1;
        }
        if handled > 0 {
            self.refresh_edge_flows();
        }
        self.persist_pending_ticks();
    }

    fn persist_pending_ticks(&mut self) {
        if self.pending_store_ticks.is_empty() || Instant::now() < self.next_store_retry {
            return;
        }
        let Some((project_id, _)) = self.pending_store_ticks.front() else {
            return;
        };
        let project_id = project_id.clone();
        let batch: Vec<_> = self
            .pending_store_ticks
            .iter()
            .take_while(|(queued_project_id, _)| queued_project_id == &project_id)
            .take(4096)
            .map(|(_, tick)| tick.clone())
            .collect();
        match self.local_store.store_ticks(&project_id, &batch) {
            Ok(()) => {
                for _ in 0..batch.len() {
                    self.pending_store_ticks.pop_front();
                }
                self.next_store_retry = Instant::now();
            }
            Err(error) => {
                self.next_store_retry = Instant::now() + std::time::Duration::from_secs(1);
                self.notice = format!(
                    "Локальное хранилище недоступно; {} обновлений ожидают записи: {error}",
                    self.pending_store_ticks.len()
                );
                eprintln!("{}", self.notice);
            }
        }
    }

    fn input_status_for(&self, node_index: usize, metric_index: usize) -> MetricStatus {
        let node = &self.project.nodes[node_index];
        let metric = &node.inputs[metric_index];
        input_status_from_flow(
            metric,
            self.input_has_connections[node_index][metric_index],
            self.input_supply[node_index][metric_index],
        )
    }

    fn output_status_for(&self, node_index: usize, metric_index: usize) -> MetricStatus {
        let node = &self.project.nodes[node_index];
        let metric = &node.outputs[metric_index];
        output_status_from_flow(
            metric,
            self.output_has_connections[node_index][metric_index],
            self.output_demand[node_index][metric_index],
            self.output_limits[node_index][metric_index],
        )
    }

    fn frame_metrics(&self) -> (f32, f32, f32, f32, f32) {
        if self.frame_samples.is_empty() {
            return (0.0, 0.0, 0.0, 0.0, 0.0);
        }
        let mut frame_times: Vec<_> = self
            .frame_samples
            .iter()
            .map(|sample| sample.frame_ms)
            .collect();
        let mut update_times: Vec<_> = self
            .frame_samples
            .iter()
            .map(|sample| sample.update_ms)
            .collect();
        frame_times.sort_by(f32::total_cmp);
        update_times.sort_by(f32::total_cmp);
        let p95 = (frame_times.len() * 95 / 100).min(frame_times.len() - 1);
        let update_p95 = (update_times.len() * 95 / 100).min(update_times.len() - 1);
        let mean = frame_times.iter().sum::<f32>() / frame_times.len() as f32;
        (
            if mean > 0.0 { 1000.0 / mean } else { 0.0 },
            frame_times[p95],
            *frame_times.last().unwrap_or(&0.0),
            update_times[update_p95],
            *update_times.last().unwrap_or(&0.0),
        )
    }

    fn diagnostic_report(&self) -> String {
        let (fps, frame_p95, frame_max, update_p95, update_max) = self.frame_metrics();
        format!(
            "Polygram QA report\nversion: {}\nos: {} {}\nrenderer: {}\nwindow focus: {}\ngraph: {} nodes, {} edges (visible: {} nodes, {} edges)\nviewport: zoom {:.0}%, frame cap {} FPS, animation {}\nstress test: {}, speed {:.1}x, steps {}, simulation {:.1}s\nframe cadence: {:.1} FPS, p95 {:.1} ms, max {:.1} ms\napp update: p95 {:.1} ms, max {:.1} ms\nstress update max: {:.2} ms\nsamples: {} (frames slower than 33 ms: {})\nwebhook queue: {}/{}\nticks: applied {}, rejected {}\n",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH,
            self.renderer,
            if self.window_focused {
                "foreground"
            } else {
                "background"
            },
            self.project.nodes.len(),
            self.project.edges.len(),
            self.visible_nodes,
            self.visible_edges,
            self.zoom * 100.0,
            self.frame_cap,
            if self.animation_enabled { "on" } else { "off" },
            if self.stress_running {
                "running"
            } else {
                "stopped"
            },
            self.stress_speed,
            self.stress_ticks,
            self.stress_elapsed,
            fps,
            frame_p95,
            frame_max,
            update_p95,
            update_max,
            self.stress_samples
                .iter()
                .map(|sample| sample.update_ms)
                .max_by(f32::total_cmp)
                .unwrap_or(0.0),
            self.frame_samples.len(),
            self.frame_samples
                .iter()
                .filter(|sample| sample.frame_ms > 33.0)
                .count(),
            self.network.ticks.len(),
            self.network.ticks.capacity(),
            self.applied_ticks,
            self.dropped_ticks
        )
    }

    fn show_qa(&mut self, ctx: &egui::Context) {
        if !self.qa_open {
            return;
        }
        let mut open = self.qa_open;
        let (fps, frame_p95, frame_max, update_p95, update_max) = self.frame_metrics();
        let report = self.diagnostic_report();
        let mut copied = false;
        egui::Window::new("QA / Диагностика отклика")
            .open(&mut open)
            .default_width(430.0)
            .show(ctx, |ui| {
                if !self.window_focused {
                    ui.colored_label(
                        MUTED,
                        "Окно не в фокусе — замер приостановлен; ОС может ограничивать кадры.",
                    );
                }
                ui.label(format!("FPS: {:.1} · ограничение: {}", fps, self.frame_cap));
                ui.label(format!("GPU: {}", self.renderer));
                ui.label(format!(
                    "Кадр p95/max: {:.1} / {:.1} мс",
                    frame_p95, frame_max
                ));
                ui.label(format!(
                    "В кадре: {} узлов, {} связей",
                    self.visible_nodes, self.visible_edges
                ));
                ui.label(format!(
                    "Обработка UI p95/max: {:.1} / {:.1} мс",
                    update_p95, update_max
                ));
                if self.stress_running || !self.stress_samples.is_empty() {
                    let stress_update_max = self
                        .stress_samples
                        .iter()
                        .map(|sample| sample.update_ms)
                        .max_by(f32::total_cmp)
                        .unwrap_or(0.0);
                    let changed_metrics = self
                        .stress_samples
                        .back()
                        .map_or(0, |sample| sample.changed_metrics);
                    ui.label(format!(
                        "Стресс-тест: {} · {:.1}× · шагов {} · метрик за кадр {}",
                        if self.stress_running {
                            "идёт"
                        } else {
                            "пауза"
                        },
                        self.stress_speed,
                        self.stress_ticks,
                        changed_metrics
                    ));
                    ui.label(format!(
                        "Пересчёт динамики max: {:.2} мс",
                        stress_update_max
                    ));
                }
                ui.label(format!(
                    "Очередь тиков: {} / {}",
                    self.network.ticks.len(),
                    self.network.ticks.capacity()
                ));
                ui.separator();
                ui.label("Тест отклика: выключите анимацию, затем перетащите блок и приблизьте холст. Включите анимацию для сравнения.");
                if ui.button("Скопировать диагностику").clicked() {
                    ctx.copy_text(report.clone());
                    copied = true;
                }
                ui.label("Отчёт не отправляется автоматически. Вставьте его сюда вместе с кратким описанием: действие → ожидание → фактический результат.");
            });
        self.qa_open = open;
        if copied {
            self.notice = "QA-отчёт скопирован в буфер обмена".to_owned();
        }
    }

    fn fit_graph(&mut self, canvas: Rect) {
        let visible_nodes: Vec<_> = self
            .project
            .nodes
            .iter()
            .filter(|node| node.parent_id == self.current_scope)
            .collect();
        let Some(first) = visible_nodes.first().copied() else {
            return;
        };
        let mut min_x = first.x;
        let mut min_y = first.y;
        let mut max_x = first.x + NODE_WIDTH;
        let mut max_y = first.y + node_height(first);
        for node in visible_nodes.into_iter().skip(1) {
            min_x = min_x.min(node.x);
            min_y = min_y.min(node.y);
            max_x = max_x.max(node.x + NODE_WIDTH);
            max_y = max_y.max(node.y + node_height(node));
        }
        let graph_size = Vec2::new(max_x - min_x, max_y - min_y);
        let fit_size = (canvas.size() - Vec2::splat(96.0)).max(Vec2::splat(32.0));
        self.zoom = (fit_size.x / graph_size.x)
            .min(fit_size.y / graph_size.y)
            .clamp(0.1, 2.0);
        self.zoom_target = self.zoom;
        self.zoom_anchor = None;
        let graph_center = Vec2::new((min_x + max_x) * 0.5, (min_y + max_y) * 0.5);
        self.camera_offset = canvas.size() * 0.5 - graph_center * self.zoom;
    }

    fn show_left_dock(&mut self, ctx: &egui::Context) {
        let icons = self
            .reference_icons
            .as_ref()
            .expect("reference icon texture loaded before drawing the dock")
            .clone();
        egui::Area::new(egui::Id::new("tools"))
            .anchor(Align2::LEFT_TOP, Vec2::new(20.0, 68.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(PANEL)
                    .stroke(Stroke::new(1.0_f32, BORDER))
                    .corner_radius(10.0)
                    .inner_margin(egui::Margin::same(4))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;
                        ui.vertical_centered(|ui| {
                            if dock_icon_button(ui, &icons, 0, self.active_tool == CanvasTool::Pan)
                                .on_hover_text("Перемещение холста")
                                .clicked()
                            {
                                self.active_tool = CanvasTool::Pan;
                                self.connecting_from = None;
                            }
                            if dock_icon_button(
                                ui,
                                &icons,
                                1,
                                self.active_tool == CanvasTool::Select,
                            )
                            .on_hover_text("Выделение и перемещение блоков")
                            .clicked()
                            {
                                self.active_tool = CanvasTool::Select;
                                self.connecting_from = None;
                            }
                            if dock_icon_button(
                                ui,
                                &icons,
                                6,
                                self.active_tool == CanvasTool::Connect,
                            )
                            .on_hover_text("Связать выход с запросом")
                            .clicked()
                            {
                                self.active_tool = CanvasTool::Connect;
                                self.connecting_from = None;
                            }
                            ui.separator();
                            let add_menu = ui
                                .menu_button("", |ui| {
                                    ui.label("Добавить на холст");
                                    ui.separator();
                                    if ui.button("Новый процесс").clicked() {
                                        self.add_process_requested = true;
                                        ui.close_menu();
                                    }
                                    if ui.button("Новый отдел").clicked() {
                                        self.add_department_requested = true;
                                        ui.close_menu();
                                    }
                                    if ui.button("Добавить портал").clicked() {
                                        self.add_portal_requested = true;
                                        ui.close_menu();
                                    }
                                    ui.separator();
                                    if ui.button("Выровнять граф").clicked() {
                                        self.project.layout_left_to_right_in_scope(
                                            self.current_scope.as_deref(),
                                        );
                                        self.rebuild_spatial();
                                        ui.close_menu();
                                    }
                                })
                                .response;
                            paint_dock_icon(
                                ui.painter(),
                                &icons,
                                2,
                                add_menu.rect.shrink(6.0),
                                if add_menu.hovered() { TEXT } else { MUTED },
                            );
                            add_menu.on_hover_text("Объекты и действия");
                            let extension_actions: Vec<_> = self
                                .extensions
                                .iter()
                                .flat_map(|extension| {
                                    extension.manifest.blocks.iter().map(|block| {
                                        (
                                            extension.manifest.id.clone(),
                                            extension.manifest.name.clone(),
                                            block.clone(),
                                        )
                                    })
                                })
                                .collect();
                            if !self.extensions.is_empty() || !self.extension_failures.is_empty() {
                                let extensions_menu = ui
                                    .menu_button("", |ui| {
                                        ui.label(format!("Расширения ({})", self.extensions.len()));
                                        if ui.button("Обновить сейчас").clicked() {
                                            self.refresh_extensions();
                                            ui.close_menu();
                                        }
                                        ui.separator();
                                        if extension_actions.is_empty() {
                                            ui.label("Пока нет доступных шаблонов");
                                        }
                                        for (extension_id, extension_name, block) in
                                            &extension_actions
                                        {
                                            let location = self
                                                .extensions
                                                .iter()
                                                .find(|extension| {
                                                    extension.manifest.id == *extension_id
                                                })
                                                .map(|extension| {
                                                    extension.directory.display().to_string()
                                                })
                                                .unwrap_or_default();
                                            if ui
                                                .button(format!(
                                                    "{} · {}",
                                                    extension_name, block.name
                                                ))
                                                .on_hover_text(location)
                                                .clicked()
                                            {
                                                self.add_extension_block(extension_id, block);
                                                ui.close_menu();
                                            }
                                        }
                                        for failure in &self.extension_failures {
                                            ui.colored_label(
                                                MUTED,
                                                format!(
                                                    "{}: {}",
                                                    failure.directory.display(),
                                                    failure.reason
                                                ),
                                            );
                                        }
                                    })
                                    .response;
                                paint_dock_icon(
                                    ui.painter(),
                                    &icons,
                                    8,
                                    extensions_menu.rect.shrink(6.0),
                                    if extensions_menu.hovered() {
                                        TEXT
                                    } else {
                                        MUTED
                                    },
                                );
                                extensions_menu.on_hover_text("Шаблоны расширений");
                            }
                            if dock_icon_button(ui, &icons, 4, false)
                                .on_hover_text("Найти процесс")
                                .clicked()
                            {
                                self.search_open = !self.search_open;
                            }
                            if dock_icon_button(ui, &icons, 3, false)
                                .on_hover_text("Вписать граф (Home)")
                                .clicked()
                            {
                                self.fit_graph_requested = true;
                            }
                        });
                        ui.menu_button(
                            if self.stress_running {
                                format!("{:.1}×", self.stress_speed)
                            } else {
                                "· · ·".to_owned()
                            },
                            |ui| {
                                ui.label("Стресс-тест");
                                if self.stress_running {
                                    ui.add(
                                        egui::Slider::new(&mut self.stress_speed, 0.1..=10.0)
                                            .text("Скорость")
                                            .fixed_decimals(1),
                                    );
                                    if ui.button("Остановить тест").clicked() {
                                        self.stress_running = false;
                                        self.notice = "Стресс-тест приостановлен".to_owned();
                                        ui.close_menu();
                                    }
                                } else if ui.button("Запустить · 1 000 узлов").clicked()
                                {
                                    self.start_stress_test();
                                    ui.close_menu();
                                }
                            },
                        )
                        .response
                        .on_hover_text("Стресс-тест");
                    });
            });

        if self.search_open {
            let mut open = self.search_open;
            egui::Window::new("Найти процесс")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.text_edit_singleline(&mut self.search_query);
                    let query = self.search_query.trim().to_lowercase();
                    let results: Vec<_> = self
                        .project
                        .nodes
                        .iter()
                        .filter(|node| {
                            node.parent_id == self.current_scope
                                && (query.is_empty()
                                    || node.name.to_lowercase().contains(&query)
                                    || node.id.to_lowercase().contains(&query))
                        })
                        .take(30)
                        .map(|node| (node.id.clone(), node.name.clone()))
                        .collect();
                    egui::ScrollArea::vertical()
                        .max_height(280.0)
                        .show(ui, |ui| {
                            for (id, name) in results {
                                if ui
                                    .selectable_label(
                                        self.selected.as_deref() == Some(id.as_str()),
                                        format!("{name} · {id}"),
                                    )
                                    .clicked()
                                {
                                    self.selected = Some(id.clone());
                                    self.selected_edge = None;
                                    self.focus_node_requested = Some(id);
                                    self.search_open = false;
                                }
                            }
                        });
                });
            self.search_open &= open;
        }
    }

    fn show_metric_dialog(&mut self, ctx: &egui::Context) {
        if ctx.input(|input| input.key_pressed(Key::Escape)) && self.metric_draft.is_some() {
            self.metric_draft = None;
            return;
        }
        let Some(draft) = self.metric_draft.as_mut() else {
            return;
        };
        let title = match draft.direction {
            MetricDirection::Input => "Добавить запрос",
            MetricDirection::Output => "Добавить выход",
        };
        let mut open = true;
        let mut close = false;
        let mut submit = false;
        egui::Window::new(title)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label(format!("Блок: {}", draft.node_id));
                ui.label("Название");
                ui.text_edit_singleline(&mut draft.name);
                ui.horizontal(|ui| {
                    ui.label("Значение");
                    ui.text_edit_singleline(&mut draft.value);
                });
                ui.horizontal(|ui| {
                    ui.label("Требуется");
                    ui.text_edit_singleline(&mut draft.required);
                });
                ui.horizontal(|ui| {
                    ui.label("Единица");
                    ui.text_edit_singleline(&mut draft.unit);
                });
                ui.horizontal(|ui| {
                    ui.label("Лимит избытка (необязательно)");
                    ui.text_edit_singleline(&mut draft.max);
                });
                if let Some(error) = &draft.error {
                    ui.colored_label(MUTED, error);
                }
                ui.horizontal(|ui| {
                    submit = ui.button("Добавить").clicked();
                    close = ui.button("Отмена").clicked();
                });
            });

        if close || !open {
            self.metric_draft = None;
        } else if submit {
            let Some(draft) = self.metric_draft.as_mut() else {
                return;
            };
            let parse_nonnegative = |text: &str| {
                let value = text.trim().parse::<f64>().ok()?;
                (value.is_finite() && value >= 0.0).then_some(value)
            };
            let value = parse_nonnegative(&draft.value);
            let required = parse_nonnegative(&draft.required);
            let max = if draft.max.trim().is_empty() {
                Some(None)
            } else {
                parse_nonnegative(&draft.max).map(Some)
            };
            let error = if draft.name.trim().is_empty() {
                Some("Укажите название показателя.".to_owned())
            } else if value.is_none() || required.is_none() || max.is_none() {
                Some("Значения должны быть конечными числами не меньше нуля.".to_owned())
            } else {
                None
            };
            if let Some(error) = error {
                draft.error = Some(error);
                return;
            }
            let node_id = draft.node_id.clone();
            let direction = draft.direction;
            let name = draft.name.trim().to_owned();
            let unit = draft.unit.trim().to_owned();
            let value = value.unwrap_or_default();
            let required = required.unwrap_or_default();
            let max = max.unwrap_or(None);
            let id = self.next_metric_id(&node_id, direction);
            if let Some(node) = self.project.node_mut(&node_id) {
                let metric = Metric {
                    id,
                    name,
                    value,
                    required,
                    max,
                    unit,
                };
                match direction {
                    MetricDirection::Input => node.inputs.push(metric),
                    MetricDirection::Output => node.outputs.push(metric),
                }
                self.selected_edge = None;
                self.rebuild_spatial();
                self.metric_draft = None;
            } else {
                self.metric_draft = None;
            }
        }
    }

    fn next_metric_id(&self, node_id: &str, direction: MetricDirection) -> String {
        let suffix = match direction {
            MetricDirection::Input => "IN",
            MetricDirection::Output => "OUT",
        };
        for index in 1_u64.. {
            let candidate = format!("{node_id}_{suffix}_{index:02}");
            let exists = self.project.nodes.iter().any(|node| {
                node.inputs
                    .iter()
                    .chain(&node.outputs)
                    .any(|metric| metric.id == candidate)
            });
            if !exists {
                return candidate;
            }
        }
        unreachable!("u64 metric id space exhausted")
    }

    fn open_metric_draft(&mut self, node_id: String, direction: MetricDirection) {
        self.metric_draft = Some(MetricDraft {
            node_id,
            direction,
            name: String::new(),
            value: "0".to_owned(),
            required: "100".to_owned(),
            unit: "ед.".to_owned(),
            max: if matches!(direction, MetricDirection::Output) {
                "500".to_owned()
            } else {
                String::new()
            },
            error: None,
        });
    }

    fn show_toolbar(&mut self, ctx: &egui::Context) {
        let logo = self
            .reference_logo
            .as_ref()
            .expect("reference logo texture loaded before drawing the header")
            .clone();
        egui::Area::new(egui::Id::new("workspace_header"))
            .anchor(Align2::LEFT_TOP, Vec2::new(20.0, 16.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(PANEL)
                    .stroke(Stroke::new(1.0_f32, BORDER))
                    .corner_radius(10.0)
                    .inner_margin(egui::Margin::symmetric(10, 4))
                    .show(ui, |ui| {
                        ui.spacing_mut().button_padding = Vec2::new(6.0, 3.0);
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::Image::new((logo.id(), Vec2::new(16.0, 18.0)))
                                    .sense(Sense::hover()),
                            );
                            ui.label(
                                egui::RichText::new("Polygram")
                                    .size(13.0)
                                    .color(TEXT)
                                    .strong(),
                            );
                            ui.separator();
                            let (dot_rect, _) =
                                ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                            ui.painter().circle_filled(dot_rect.center(), 3.0, AMBER);
                            if let Some(scope_id) = self.current_scope.clone() {
                                let name = self
                                    .project
                                    .node(&scope_id)
                                    .map_or("Моя миссия", |node| node.name.as_str())
                                    .to_owned();
                                if ui.button(name).clicked() {
                                    self.go_to_parent_scope();
                                }
                            } else {
                                ui.label("Моя миссия");
                            }
                            if self.current_scope.is_some() {
                                ui.separator();
                                if ui.button("Home").on_hover_text("Корневой граф").clicked()
                                {
                                    self.current_scope = None;
                                    self.selected = None;
                                    self.fit_graph_requested = true;
                                }
                            }
                            ui.separator();
                            ui.menu_button("Файл", |ui| {
                                if ui.button("Открыть .pg").clicked() {
                                    self.load_project();
                                    ui.close_menu();
                                }
                                if ui.button("Сохранить .pg").clicked() {
                                    self.save_project();
                                    ui.close_menu();
                                }
                            });
                            ui.menu_button("...", |ui| {
                                if ui.button("Выровнять граф").clicked() {
                                    self.project.layout_left_to_right_in_scope(
                                        self.current_scope.as_deref(),
                                    );
                                    self.rebuild_spatial();
                                    ui.close_menu();
                                }
                                if ui.button("Вписать граф").clicked() {
                                    self.fit_graph_requested = true;
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui.button("1 000 узлов").clicked() {
                                    self.generate_sample(1_000);
                                    ui.close_menu();
                                }
                                if ui.button("100 узлов").clicked() {
                                    self.generate_sample(100);
                                    ui.close_menu();
                                }
                            });
                        });
                    });
            });

        egui::Area::new(egui::Id::new("workspace_status"))
            .anchor(Align2::RIGHT_TOP, Vec2::new(-20.0, 16.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(PANEL)
                    .stroke(Stroke::new(1.0_f32, BORDER))
                    .corner_radius(10.0)
                    .inner_margin(egui::Margin::symmetric(6, 4))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.menu_button(self.frame_cap.to_string(), |ui| {
                                for cap in [30, 60, 120, 144] {
                                    if ui
                                        .selectable_value(
                                            &mut self.frame_cap,
                                            cap,
                                            format!("{cap} FPS"),
                                        )
                                        .clicked()
                                    {
                                        ui.close_menu();
                                    }
                                }
                            });
                            ui.checkbox(&mut self.animation_enabled, "A")
                                .on_hover_text("Анимация");
                            if ui
                                .button("QA")
                                .on_hover_text("Диагностика отклика")
                                .clicked()
                            {
                                self.qa_open = true;
                            }
                        });
                    });
            });

        egui::Area::new(egui::Id::new("canvas_zoom_controls"))
            .anchor(Align2::LEFT_BOTTOM, Vec2::new(20.0, -20.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(PANEL)
                    .stroke(Stroke::new(1.0_f32, BORDER))
                    .corner_radius(10.0)
                    .inner_margin(egui::Margin::symmetric(8, 5))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if ui.button("−").on_hover_text("Отдалить").clicked() {
                                self.zoom_step_requested = -1.0;
                            }
                            if ui
                                .button(format!("{:.0}%", self.zoom * 100.0))
                                .on_hover_text("Масштаб")
                                .clicked()
                            {
                                self.zoom_reset_requested = true;
                            }
                            if ui.button("+").on_hover_text("Приблизить").clicked() {
                                self.zoom_step_requested = 1.0;
                            }
                            ui.separator();
                            if ui.button("Сброс").clicked() {
                                self.fit_graph_requested = true;
                            }
                            ui.separator();
                            ui.label(format!("{} узлов", self.project.nodes.len()));
                        });
                    });
            });

        if self.notice != "Локальный webhook: http://localhost:8080/api/v1/tick" {
            egui::Area::new(egui::Id::new("workspace_notice"))
                .anchor(Align2::RIGHT_BOTTOM, Vec2::new(-24.0, -20.0))
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::new()
                        .fill(PANEL)
                        .stroke(Stroke::new(1.0_f32, BORDER))
                        .corner_radius(12.0)
                        .inner_margin(egui::Margin::symmetric(12, 8))
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new(&self.notice).small().color(MUTED));
                        });
                });
        }
    }

    fn show_inspector(&mut self, ctx: &egui::Context) {
        if self.selected.is_none() && self.selected_edge.is_none() {
            return;
        }
        let mut delete_edge = false;
        egui::SidePanel::right("inspector")
            .resizable(true)
            .default_width(300.0)
            .min_width(260.0)
            .frame(
                egui::Frame::new()
                    .fill(PANEL)
                    .stroke(Stroke::new(1.0_f32, BORDER))
                    .inner_margin(14.0),
            )
            .show(ctx, |ui| {
                ui.heading("Инспектор");
                ui.add_space(10.0);
                if let Some(edge_index) = self.selected_edge {
                    if let Some(edge) = self.project.edges.get(edge_index) {
                        let source = self.project.node(&edge.from_node);
                        let target = self.project.node(&edge.to_node);
                        let metric = source.and_then(|node| {
                            node.outputs
                                .iter()
                                .find(|metric| metric.id == edge.from_metric)
                        });
                        ui.label("Связь показателей");
                        ui.separator();
                        ui.label(format!(
                            "{} → {}",
                            source.map_or(edge.from_node.as_str(), |node| node.name.as_str()),
                            target.map_or(edge.to_node.as_str(), |node| node.name.as_str())
                        ));
                        if let Some(metric) = metric {
                            ui.label(format!("{} · {}", metric.name, metric.unit));
                        }
                        if let Some(flow) = self.edge_allocations.get(edge_index) {
                            ui.label(format!("Передаётся: {}", format_number(*flow)));
                        }
                        delete_edge = ui
                            .button(egui::RichText::new("Удалить связь").color(MUTED))
                            .clicked();
                    }
                    if delete_edge {
                        self.project.edges.remove(edge_index);
                        self.selected_edge = None;
                        self.rebuild_render_indexes();
                    }
                    return;
                }
                let Some(selected) = self.selected.clone() else {
                    ui.label(egui::RichText::new("Выберите блок или связь").color(MUTED));
                    ui.add_space(12.0);
                    ui.label(
                        "Связи создаются инструментом «Связь»: выберите выход, затем запрос другого блока.",
                    );
                    return;
                };
                let mut delete = false;
                let mut add_input = false;
                let mut add_output = false;
                let mut poll_config_changed = false;
                let mut render_limits_changed = false;
                let mut flow_changed = false;
                let selected_is_department = self
                    .project
                    .node(&selected)
                    .is_some_and(|node| node.is_department);
                let selected_is_portal = self
                    .project
                    .node(&selected)
                    .is_some_and(|node| node.is_portal);
                let selected_index = self.node_indices.get(&selected).copied().unwrap_or_default();
                if let Some(node) = self.project.node_mut(&selected) {
                    ui.label(egui::RichText::new(&node.id).small().color(MUTED));
                    ui.label("Название");
                    ui.text_edit_singleline(&mut node.name);
                    ui.add_space(8.0);
                    ui.label("Тип");
                    ui.text_edit_singleline(&mut node.kind);
                    ui.separator();
                    ui.heading("REST polling");
                    ui.label("URL возвращает JSON тика: node_id, metric_id, value");
                    poll_config_changed |= ui.text_edit_singleline(&mut node.poll_url).changed();
                    ui.horizontal(|ui| {
                        ui.label("Интервал, сек.");
                        poll_config_changed |= ui
                            .add(egui::DragValue::new(&mut node.poll_interval_secs).range(1..=86400))
                            .changed();
                    });
                    ui.add_space(8.0);
                    ui.label(format!("Входных показателей: {}", node.inputs.len()));
                    for (metric_index, metric) in node.inputs.iter_mut().enumerate() {
                        ui.label(format!("{} · #{}", metric.name, metric.id));
                        ui.horizontal(|ui| {
                            ui.label("Требование");
                            flow_changed |= ui.add(
                                egui::DragValue::new(&mut metric.required)
                                    .speed(1.0)
                                    .range(0.0..=f64::MAX),
                            ).changed();
                        });
                        let mut has_max = metric.max.is_some();
                        ui.horizontal(|ui| {
                            render_limits_changed |=
                                ui.checkbox(&mut has_max, "Целевой max").changed();
                            if has_max {
                                let max = metric.max.get_or_insert(500.0);
                                render_limits_changed |= ui
                                    .add(
                                    egui::DragValue::new(max)
                                        .speed(1.0)
                                        .range(0.0..=f64::MAX),
                                    )
                                    .changed();
                            }
                        });
                        if !has_max {
                            metric.max = None;
                        }
                        let connected = self.input_has_connections[selected_index][metric_index];
                        let supplied = if connected {
                            self.input_supply[selected_index][metric_index]
                        } else {
                            metric.value
                        };
                        let status = input_status_from_flow(metric, connected, supplied);
                        ui.label(
                            egui::RichText::new(format!(
                                "{}: {} {}",
                                if connected {
                                    "Поступило"
                                } else {
                                    "Текущее значение"
                                },
                                format_number(supplied),
                                metric.unit
                            ))
                            .small()
                            .color(status_color(status)),
                        )
                        .on_hover_text(metric_status_detail(
                            status,
                            supplied,
                            metric.required,
                            &metric.unit,
                        ));
                    }
                    ui.add_space(4.0);
                    ui.label(format!("Выходных показателей: {}", node.outputs.len()));
                    for (metric_index, metric) in node.outputs.iter_mut().enumerate() {
                        ui.label(format!("{} · #{}", metric.name, metric.id));
                        let max = metric.max.get_or_insert(1_000.0);
                        ui.horizontal(|ui| {
                            ui.label("Лимит max");
                            render_limits_changed |= ui
                                .add(
                                egui::DragValue::new(max)
                                    .speed(1.0)
                                    .range(0.0..=f64::MAX),
                                )
                                .changed();
                        });
                        let connected = self.output_has_connections[selected_index][metric_index];
                        let demand = if connected {
                            self.output_demand[selected_index][metric_index]
                        } else {
                            metric.required
                        };
                        let status = output_status_from_flow(
                            metric,
                            connected,
                            demand,
                            self.output_limits[selected_index][metric_index],
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "Текущее значение: {} {}",
                                format_number(metric.value),
                                metric.unit
                            ))
                            .small()
                            .color(status_color(status)),
                        )
                        .on_hover_text(metric_status_detail(
                            status,
                            metric.value,
                            demand,
                            &metric.unit,
                        ));
                    }
                    ui.add_space(12.0);
                    add_input = ui.button("＋ Добавить запрос").clicked();
                    add_output = ui.button("＋ Добавить выход").clicked();
                    delete = ui
                        .button(egui::RichText::new("Удалить блок").color(MUTED))
                        .clicked();
                }
                if delete {
                    let mut removed = vec![selected.clone()];
                    let mut index = 0;
                    while index < removed.len() {
                        let parent = removed[index].clone();
                        removed.extend(
                            self.project
                                .nodes
                                .iter()
                                .filter(|node| node.parent_id.as_deref() == Some(parent.as_str()))
                                .map(|node| node.id.clone()),
                        );
                        index += 1;
                    }
                    self.project
                        .nodes
                        .retain(|node| !removed.iter().any(|id| id == &node.id));
                    self.project
                        .edges
                        .retain(|edge| !removed.contains(&edge.from_node) && !removed.contains(&edge.to_node));
                    self.selected = None;
                    self.selected_edge = None;
                    self.rebuild_spatial();
                    self.sync_polling();
                } else {
                    if add_input || add_output {
                        self.open_metric_draft(
                            selected.clone(),
                            if add_input {
                                MetricDirection::Input
                            } else {
                                MetricDirection::Output
                            },
                        );
                    } else if render_limits_changed {
                        self.rebuild_render_indexes();
                    } else if flow_changed {
                        self.refresh_edge_flows();
                    }
                    if poll_config_changed || delete {
                        self.sync_polling();
                    }
                    if selected_is_department {
                        let child_count = self
                            .project
                            .nodes
                            .iter()
                            .filter(|node| node.parent_id.as_deref() == Some(selected.as_str()))
                            .count();
                        ui.separator();
                        ui.label(format!("Вложенных блоков: {child_count}"));
                        if ui.button("Открыть отдел →").clicked() {
                            self.enter_department(&selected);
                        }
                    }
                    if selected_is_portal {
                        ui.separator();
                        ui.heading("Межуровневые связи");
                        ui.label("Портал передаёт входящий поток на свой выход.");
                        let external_outputs: Vec<_> = self
                            .project
                            .nodes
                            .iter()
                            .filter(|node| node.parent_id != self.current_scope && !node.is_department)
                            .flat_map(|node| {
                                node.outputs.iter().map(|metric| {
                                    (
                                        node.id.clone(),
                                        node.name.clone(),
                                        metric.id.clone(),
                                        metric.name.clone(),
                                    )
                                })
                            })
                            .collect();
                        for (node_id, node_name, metric_id, metric_name) in external_outputs {
                            if ui
                                .button(format!("Вход ← {node_name} · {metric_name}"))
                                .clicked()
                            {
                                let portal_input = self
                                    .project
                                    .node(&selected)
                                    .and_then(|node| node.inputs.first())
                                    .map(|metric| metric.id.clone())
                                    .unwrap_or_default();
                                self.add_portal_edge(
                                    &node_id,
                                    &metric_id,
                                    &selected,
                                    &portal_input,
                                );
                            }
                        }
                        let external_inputs: Vec<_> = self
                            .project
                            .nodes
                            .iter()
                            .filter(|node| node.parent_id != self.current_scope && !node.is_department)
                            .flat_map(|node| {
                                node.inputs.iter().map(|metric| {
                                    (
                                        node.id.clone(),
                                        node.name.clone(),
                                        metric.id.clone(),
                                        metric.name.clone(),
                                    )
                                })
                            })
                            .collect();
                        for (node_id, node_name, metric_id, metric_name) in external_inputs {
                            if ui
                                .button(format!("Выход → {node_name} · {metric_name}"))
                                .clicked()
                            {
                                let portal_output = self
                                    .project
                                    .node(&selected)
                                    .and_then(|node| node.outputs.first())
                                    .map(|metric| metric.id.clone())
                                    .unwrap_or_default();
                                self.add_portal_edge(
                                    &selected,
                                    &portal_output,
                                    &node_id,
                                    &metric_id,
                                );
                            }
                        }
                    }
                }
            });
    }

    fn add_portal_edge(
        &mut self,
        source_node: &str,
        source_metric: &str,
        target_node: &str,
        target_metric: &str,
    ) {
        let edge = Edge {
            from_node: source_node.to_owned(),
            from_metric: source_metric.to_owned(),
            to_node: target_node.to_owned(),
            to_metric: target_metric.to_owned(),
        };
        if self.project.edges.iter().any(|existing| {
            existing.from_node == edge.from_node
                && existing.from_metric == edge.from_metric
                && existing.to_node == edge.to_node
                && existing.to_metric == edge.to_metric
        }) {
            self.notice = "Такая связь уже существует".to_owned();
            return;
        }
        self.project.edges.push(edge);
        self.rebuild_render_indexes();
        self.notice = "Межуровневая связь через портал создана".to_owned();
    }

    fn show_canvas(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG))
            .show(ctx, |ui| {
                let rect = ui.available_rect_before_wrap();
                if self.zoom_reset_requested || self.zoom_step_requested != 0.0 {
                    let center = rect.center();
                    let world = self.screen_to_world(rect, center);
                    self.zoom_target = if self.zoom_reset_requested {
                        1.0
                    } else {
                        (self.zoom_target * 1.2_f32.powf(self.zoom_step_requested)).clamp(0.1, 5.0)
                    };
                    self.zoom_anchor = Some((center - rect.min, Pos2::new(world.x, world.y)));
                    self.zoom_step_requested = 0.0;
                    self.zoom_reset_requested = false;
                }
                if self.add_process_requested {
                    self.add_process_requested = false;
                    self.add_node(self.screen_to_world(rect, rect.center()));
                    self.active_tool = CanvasTool::Select;
                }
                if self.add_department_requested {
                    self.add_department_requested = false;
                    self.add_department(self.screen_to_world(rect, rect.center()));
                    self.active_tool = CanvasTool::Select;
                }
                if self.add_portal_requested {
                    self.add_portal_requested = false;
                    self.add_portal(self.screen_to_world(rect, rect.center()));
                    self.active_tool = CanvasTool::Select;
                }
                if let Some(node_id) = self.focus_node_requested.take()
                    && let Some(node) = self.project.node(&node_id)
                {
                    self.zoom_anchor = None;
                    self.zoom_target = self.zoom;
                    self.camera_offset = rect.size() * 0.5
                        - Vec2::new(node.x + NODE_WIDTH * 0.5, node.y + node_height(node) * 0.5)
                            * self.zoom;
                }
                if self.fit_graph_requested || ctx.input(|input| input.key_pressed(Key::Home)) {
                    self.fit_graph_requested = false;
                    self.fit_graph(rect);
                }
                let response = ui.allocate_rect(rect, Sense::click_and_drag());
                let painter = ui.painter().with_clip_rect(rect);
                let pointer = ctx.input(|input| input.pointer.interact_pos());
                let pointer_pressed =
                    ctx.input(|input| input.pointer.button_pressed(PointerButton::Primary));
                let secondary_pressed =
                    ctx.input(|input| input.pointer.button_pressed(PointerButton::Secondary));
                let pointer_down =
                    ctx.input(|input| input.pointer.button_down(PointerButton::Primary));
                let double_clicked =
                    ctx.input(|input| input.pointer.button_double_clicked(PointerButton::Primary));
                let pointer_delta = ctx.input(|input| input.pointer.delta());
                let is_panning = ctx.input(|input| {
                    input.pointer.button_down(PointerButton::Middle)
                        || input.pointer.button_down(PointerButton::Secondary)
                        || (self.active_tool == CanvasTool::Pan && pointer_down)
                });

                if response.hovered() && is_panning {
                    self.camera_offset += pointer_delta;
                } else if pointer_down && self.active_tool == CanvasTool::Select {
                    if let Some(node_id) = self.dragging_node.clone()
                        && pointer_delta != Vec2::ZERO
                    {
                        self.move_node(&node_id, pointer_delta / self.zoom);
                    }
                } else if !pointer_down {
                    self.dragging_node = None;
                }

                if response.hovered()
                    && let Some(cursor) = ctx.input(|input| input.pointer.hover_pos())
                {
                    let scroll_delta = ctx.input(|input| input.raw_scroll_delta.y);
                    if scroll_delta != 0.0 {
                        let world = self.screen_to_world(rect, cursor);
                        let next_zoom =
                            (self.zoom_target * 1.0015_f32.powf(scroll_delta)).clamp(0.1, 5.0);
                        if next_zoom != self.zoom_target {
                            self.zoom_target = next_zoom;
                            self.zoom_anchor =
                                Some((cursor - rect.min, Pos2::new(world.x, world.y)));
                        }
                    }
                }
                self.advance_zoom(ctx.input(|input| input.stable_dt));

                self.draw_grid(&painter, rect);
                self.visible_edges = self.draw_edges(&painter, rect, ctx.input(|input| input.time));

                let viewport = self.screen_to_world(rect, rect.min);
                let viewport_end = self.screen_to_world(rect, rect.max);
                let query = rstar::AABB::from_corners(
                    [
                        viewport.x.min(viewport_end.x) as f64,
                        viewport.y.min(viewport_end.y) as f64,
                    ],
                    [
                        viewport.x.max(viewport_end.x) as f64,
                        viewport.y.max(viewport_end.y) as f64,
                    ],
                );
                let visible_indices: Vec<usize> = self
                    .spatial
                    .locate_in_envelope_intersecting(&query)
                    .filter_map(|entry| self.node_indices.get(&entry.id).copied())
                    .filter(|&index| self.project.nodes[index].parent_id == self.current_scope)
                    .collect();
                self.visible_nodes = visible_indices.len();
                if secondary_pressed
                    && response.hovered()
                    && let Some(position) = ctx.input(|input| input.pointer.interact_pos())
                {
                    self.context_menu_world = Some(self.screen_to_world(rect, position));
                    self.context_menu_node = visible_indices
                        .iter()
                        .find(|&&index| {
                            self.node_screen_rect(rect, &self.project.nodes[index])
                                .contains(position)
                        })
                        .map(|&index| self.project.nodes[index].id.clone());
                    if let Some(node_id) = self.context_menu_node.clone() {
                        self.selected = Some(node_id);
                        self.selected_edge = None;
                    }
                }
                let mut anchors = Vec::new();
                for &node_index in &visible_indices {
                    self.draw_node(&painter, rect, node_index, &mut anchors);
                }

                let cursor_over_node = pointer.is_some_and(|point| {
                    visible_indices.iter().any(|&index| {
                        self.node_screen_rect(rect, &self.project.nodes[index])
                            .contains(point)
                    })
                });

                if pointer_pressed
                    && response.hovered()
                    && let Some(position) = pointer
                {
                    let clicked_anchor = anchors
                        .iter()
                        .find(|(anchor, _, _, _)| anchor.contains(position))
                        .map(|(_, node, metric, input)| (*node, *metric, *input));
                    let clicked_node = visible_indices
                        .iter()
                        .find(|&&index| {
                            self.node_screen_rect(rect, &self.project.nodes[index])
                                .contains(position)
                        })
                        .copied();

                    if self.active_tool == CanvasTool::Connect {
                        if let Some((node_index, metric_index, is_input)) = clicked_anchor {
                            let node_id = self.project.nodes[node_index].id.clone();
                            if is_input {
                                if let Some((from_node, from_metric)) = self.connecting_from.take()
                                {
                                    let target_metric = self.project.nodes[node_index].inputs
                                        [metric_index]
                                        .id
                                        .clone();
                                    if from_node == node_id {
                                        self.notice =
                                            "Нельзя связать блок с самим собой".to_owned();
                                    } else if self.project.edges.iter().any(|edge| {
                                        edge.from_node == from_node
                                            && edge.from_metric == from_metric
                                            && edge.to_node == node_id
                                            && edge.to_metric == target_metric
                                    }) {
                                        self.notice = "Такая связь уже существует".to_owned();
                                    } else {
                                        self.project.edges.push(Edge {
                                            from_node,
                                            from_metric,
                                            to_node: node_id.clone(),
                                            to_metric: target_metric,
                                        });
                                        self.selected = Some(node_id);
                                        self.selected_edge = None;
                                        self.rebuild_render_indexes();
                                        self.notice = "Связь создана".to_owned();
                                    }
                                } else {
                                    self.notice = "Сначала выберите выходной показатель".to_owned();
                                }
                            } else {
                                self.connecting_from = Some((
                                    node_id.clone(),
                                    self.project.nodes[node_index].outputs[metric_index]
                                        .id
                                        .clone(),
                                ));
                                self.selected = Some(node_id);
                                self.selected_edge = None;
                                self.notice = "Теперь выберите запрос другого блока".to_owned();
                            }
                        } else if let Some(node_index) = clicked_node {
                            self.selected = Some(self.project.nodes[node_index].id.clone());
                            self.selected_edge = None;
                        }
                    } else if self.active_tool == CanvasTool::Select {
                        if let Some((node_index, direction)) =
                            self.metric_add_target_at(rect, position, &visible_indices)
                        {
                            let node_id = self.project.nodes[node_index].id.clone();
                            self.selected = Some(node_id.clone());
                            self.selected_edge = None;
                            self.open_metric_draft(node_id, direction);
                        } else if let Some(node_index) = clicked_node {
                            let node_id = self.project.nodes[node_index].id.clone();
                            let is_department = self.project.nodes[node_index].is_department;
                            if double_clicked && is_department {
                                self.enter_department(&node_id);
                            } else {
                                self.selected = Some(node_id.clone());
                                self.selected_edge = None;
                                self.dragging_node = Some(node_id);
                            }
                        } else if let Some(edge_index) = self.edge_at(rect, position) {
                            self.selected = None;
                            self.selected_edge = Some(edge_index);
                        } else if double_clicked {
                            self.selected = None;
                            self.selected_edge = None;
                            self.add_node(self.screen_to_world(rect, position));
                        } else {
                            self.selected = None;
                            self.selected_edge = None;
                        }
                    }
                }

                if self.connecting_from.is_some()
                    && let Some(cursor) = pointer
                {
                    painter.circle_filled(cursor, 5.0, BLUE);
                    painter.text(
                        rect.left_bottom() + Vec2::new(14.0, -18.0),
                        Align2::LEFT_BOTTOM,
                        "Выберите входящий показатель",
                        FontId::proportional(12.0),
                        BLUE,
                    );
                }
                response.context_menu(|ui| {
                    if let Some(node_id) = self.context_menu_node.clone()
                        && self
                            .project
                            .node(&node_id)
                            .is_some_and(|node| node.is_department)
                    {
                        if ui.button("Открыть отдел").clicked() {
                            self.enter_department(&node_id);
                            ui.close_menu();
                        }
                        ui.separator();
                    }
                    let world = self
                        .context_menu_world
                        .unwrap_or_else(|| self.screen_to_world(rect, rect.center()));
                    if ui.button("＋ Добавить процесс здесь").clicked() {
                        self.add_node(world);
                        ui.close_menu();
                    }
                    if ui.button("▦ Создать отдел здесь").clicked() {
                        self.add_department(world);
                        ui.close_menu();
                    }
                    if ui.button("Добавить портал здесь").clicked() {
                        self.add_portal(world);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Вписать граф").clicked() {
                        self.fit_graph_requested = true;
                        ui.close_menu();
                    }
                    if ui.button("Выровнять граф").clicked() {
                        self.project
                            .layout_left_to_right_in_scope(self.current_scope.as_deref());
                        self.rebuild_spatial();
                        ui.close_menu();
                    }
                });
                ctx.set_cursor_icon(if is_panning {
                    egui::CursorIcon::Grabbing
                } else if cursor_over_node {
                    egui::CursorIcon::PointingHand
                } else {
                    match self.active_tool {
                        CanvasTool::Select => egui::CursorIcon::Default,
                        CanvasTool::Pan => egui::CursorIcon::Grab,
                        CanvasTool::Connect => egui::CursorIcon::Crosshair,
                    }
                });
            });
    }

    fn move_node(&mut self, id: &str, delta: Vec2) {
        let old_entry = self.project.node(id).map(SpatialEntry::for_node);
        if let Some(old_entry) = old_entry {
            self.spatial.remove(&old_entry);
            if let Some(node) = self.project.node_mut(id) {
                node.x += delta.x;
                node.y += delta.y;
                self.spatial.insert(SpatialEntry::for_node(node));
            }
        }
    }

    fn advance_zoom(&mut self, dt: f32) {
        let delta = self.zoom_target - self.zoom;
        if delta.abs() < 0.001 {
            self.zoom = self.zoom_target;
            if let Some((screen_anchor, world_anchor)) = self.zoom_anchor.take() {
                self.camera_offset =
                    screen_anchor - Vec2::new(world_anchor.x, world_anchor.y) * self.zoom;
            }
            return;
        }

        let smoothing = 1.0 - (-18.0 * dt.clamp(0.0, 0.1)).exp();
        self.zoom += delta * smoothing;
        if let Some((screen_anchor, world_anchor)) = self.zoom_anchor {
            self.camera_offset =
                screen_anchor - Vec2::new(world_anchor.x, world_anchor.y) * self.zoom;
        }
    }

    fn screen_to_world(&self, rect: Rect, screen: Pos2) -> Pos2 {
        let world = (screen - rect.min - self.camera_offset) / self.zoom;
        Pos2::new(world.x, world.y)
    }

    fn world_to_screen(&self, rect: Rect, world: Pos2) -> Pos2 {
        rect.min + self.camera_offset + Vec2::new(world.x, world.y) * self.zoom
    }

    fn node_screen_rect(&self, canvas: Rect, node: &Node) -> Rect {
        let min = self.world_to_screen(canvas, Pos2::new(node.x, node.y));
        Rect::from_min_size(min, Vec2::new(NODE_WIDTH, node_height(node)) * self.zoom)
    }

    fn metric_add_target_at(
        &self,
        canvas: Rect,
        pointer: Pos2,
        visible_indices: &[usize],
    ) -> Option<(usize, MetricDirection)> {
        for &node_index in visible_indices {
            let node = &self.project.nodes[node_index];
            let card = self.node_screen_rect(canvas, node);
            let mut section_top = card.top() + 50.0 * self.zoom;
            for (direction, metrics) in [
                (MetricDirection::Input, &node.inputs),
                (MetricDirection::Output, &node.outputs),
            ] {
                let section_height =
                    (92.0 + metrics.len().saturating_sub(1) as f32 * 38.0) * self.zoom;
                let section = Rect::from_min_size(
                    Pos2::new(card.left() + 9.0 * self.zoom, section_top),
                    Vec2::new(card.width() - 18.0 * self.zoom, section_height),
                );
                let heading_add = Rect::from_center_size(
                    Pos2::new(
                        section.right() - 13.0 * self.zoom,
                        section.top() + 14.0 * self.zoom,
                    ),
                    Vec2::splat(18.0 * self.zoom),
                );
                let add_area = Rect::from_min_max(
                    Pos2::new(
                        section.left() + 7.0 * self.zoom,
                        section.bottom() - 18.0 * self.zoom,
                    ),
                    Pos2::new(
                        section.right() - 7.0 * self.zoom,
                        section.bottom() - 5.0 * self.zoom,
                    ),
                );
                if add_area.contains(pointer) || heading_add.contains(pointer) {
                    return Some((node_index, direction));
                }
                section_top += section_height;
                if matches!(direction, MetricDirection::Input) {
                    section_top += 7.0 * self.zoom;
                }
            }
        }
        None
    }

    fn draw_grid(&self, painter: &egui::Painter, rect: Rect) {
        painter.rect_filled(rect, 0.0, BG);
        let gradient_steps = 24;
        let gradient_radius = rect.size().length() * 0.56;
        for step in 0..=gradient_steps {
            let amount = step as f32 / gradient_steps as f32;
            let shade = |base: u8, highlight: u8| {
                (base as f32 + (highlight as f32 - base as f32) * amount).round() as u8
            };
            painter.circle_filled(
                rect.center(),
                gradient_radius * (1.0 - amount),
                Color32::from_rgb(shade(8, 12), shade(8, 13), shade(10, 17)),
            );
        }
        let spacing = (18.0 * self.zoom).max(12.0);
        let start_x = rect.left() + self.camera_offset.x.rem_euclid(spacing);
        let start_y = rect.top() + self.camera_offset.y.rem_euclid(spacing);
        let mut x = start_x;
        while x < rect.right() {
            let mut y = start_y;
            while y < rect.bottom() {
                painter.circle_filled(Pos2::new(x, y), 0.9, Color32::from_gray(18));
                y += spacing;
            }
            x += spacing;
        }
    }

    fn edge_at(&self, canvas: Rect, pointer: Pos2) -> Option<usize> {
        let mut nearest = None;
        let mut nearest_distance = 9.0;
        for (edge_index, edge) in self.resolved_edges.iter().enumerate() {
            if !self.edge_visible_in_scope(edge) {
                continue;
            }
            let from_node = &self.project.nodes[edge.from_node];
            let to_node = &self.project.nodes[edge.to_node];
            let p0 = self.world_to_screen(
                canvas,
                Pos2::new(
                    from_node.x + NODE_WIDTH,
                    metric_y(from_node, false, edge.from_metric),
                ),
            );
            let p3 = self.world_to_screen(
                canvas,
                Pos2::new(to_node.x, metric_y(to_node, true, edge.to_metric)),
            );
            let control = (p3.x - p0.x).abs().max(80.0 * self.zoom) * 0.48;
            let p1 = p0 + Vec2::new(control, 0.0);
            let direction = if p3.x >= p0.x { 1.0 } else { -1.0 };
            let p2 = p3 - Vec2::new(control * direction, 0.0);
            let mut previous = p0;
            for step in 1..=24 {
                let current = cubic_bezier(p0, p1, p2, p3, step as f32 / 24.0);
                let distance = distance_to_segment(pointer, previous, current);
                if distance < nearest_distance {
                    nearest_distance = distance;
                    nearest = Some(edge_index);
                }
                previous = current;
            }
        }
        nearest
    }

    fn edge_visible_in_scope(&self, edge: &ResolvedEdge) -> bool {
        let source = &self.project.nodes[edge.from_node];
        let target = &self.project.nodes[edge.to_node];
        let source_local = source.parent_id == self.current_scope;
        let target_local = target.parent_id == self.current_scope;
        (source_local && target_local)
            || (source_local && source.is_portal)
            || (target_local && target.is_portal)
    }

    fn draw_edges(&self, painter: &egui::Painter, canvas: Rect, time: f64) -> usize {
        let mut drawn_edges = 0;
        for (edge_index, edge) in self.resolved_edges.iter().enumerate() {
            if !self.edge_visible_in_scope(edge) {
                continue;
            }
            let from_node = &self.project.nodes[edge.from_node];
            let to_node = &self.project.nodes[edge.to_node];
            let metric = &from_node.outputs[edge.from_metric];
            let from_y = metric_y(from_node, false, edge.from_metric);
            let to_y = metric_y(to_node, true, edge.to_metric);
            let p0 = self.world_to_screen(canvas, Pos2::new(from_node.x + NODE_WIDTH, from_y));
            let p3 = self.world_to_screen(canvas, Pos2::new(to_node.x, to_y));
            let control = (p3.x - p0.x).abs().max(80.0 * self.zoom) * 0.48;
            let bounds = Rect::from_two_pos(p0, p3).expand(control + 20.0 * self.zoom);
            if !bounds.intersects(canvas) {
                continue;
            }
            drawn_edges += 1;
            let p1 = p0 + Vec2::new(control, 0.0);
            let direction = if p3.x >= p0.x { 1.0 } else { -1.0 };
            let p2 = p3 - Vec2::new(control * direction, 0.0);
            let target = &to_node.inputs[edge.to_metric];
            let allocated = self.edge_allocations[edge_index];
            let color = if self.selected_edge == Some(edge_index) {
                BLUE
            } else if allocated + 0.0001 < target.required {
                RED
            } else {
                status_color(self.output_status_for(edge.from_node, edge.from_metric))
            };
            let points: Vec<_> = (0..=24)
                .map(|step| cubic_bezier(p0, p1, p2, p3, step as f32 / 24.0))
                .collect();
            painter.add(egui::Shape::line(
                points,
                Stroke::new(
                    if self.selected_edge == Some(edge_index) {
                        2.6
                    } else {
                        1.7
                    } * self.zoom.clamp(0.65, 1.5),
                    color.gamma_multiply(0.78),
                ),
            ));
            if self.selected_edge == Some(edge_index) {
                let midpoint = cubic_bezier(p0, p1, p2, p3, 0.5);
                let text = format!("{} {}", format_number(allocated), metric.unit);
                painter.text(
                    midpoint,
                    Align2::CENTER_CENTER,
                    text,
                    FontId::proportional(11.0),
                    TEXT,
                );
            }
            if self.animation_enabled && self.zoom > 0.25 && metric.value > 0.0 {
                let particle =
                    ((time * (0.18 + metric.value.abs().log10().max(0.0) * 0.025)).fract()) as f32;
                painter.circle_filled(cubic_bezier(p0, p1, p2, p3, particle), 3.3, color);
                painter.circle_filled(
                    cubic_bezier(p0, p1, p2, p3, (particle + 0.5).fract()),
                    2.0,
                    color.gamma_multiply(0.7),
                );
            }
        }
        drawn_edges
    }

    fn draw_node(
        &self,
        painter: &egui::Painter,
        canvas: Rect,
        node_index: usize,
        anchors: &mut Vec<(Rect, usize, usize, bool)>,
    ) {
        let node = &self.project.nodes[node_index];
        let rect = self.node_screen_rect(canvas, node);
        let header_fill = if node.is_department {
            Color32::from_rgb(31, 32, 39)
        } else if node.is_portal {
            Color32::from_rgb(28, 32, 39)
        } else {
            Color32::from_rgb(28, 29, 36)
        };
        painter.rect_filled(rect, 11.0, BORDER);
        painter.rect_filled(rect.shrink(1.0 * self.zoom), 10.0, CARD);
        painter.rect_filled(
            Rect::from_min_size(rect.min, Vec2::new(rect.width(), 40.0 * self.zoom)),
            10.0,
            header_fill,
        );
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(rect.left(), rect.top() + 30.0 * self.zoom),
                Pos2::new(rect.right(), rect.top() + 41.0 * self.zoom),
            ),
            0.0,
            header_fill,
        );
        if self.selected.as_deref() == Some(node.id.as_str()) {
            painter.rect_stroke(
                rect,
                11.0,
                Stroke::new(1.5_f32, BORDER_ACTIVE),
                egui::StrokeKind::Inside,
            );
        }
        painter.text(
            rect.min + Vec2::new(13.0, 19.0) * self.zoom,
            Align2::LEFT_CENTER,
            format!("{} · {}", node.kind, node.name),
            FontId::proportional(14.0 * self.zoom),
            TEXT,
        );
        painter.text(
            Pos2::new(
                rect.right() - 12.0 * self.zoom,
                rect.top() + 19.0 * self.zoom,
            ),
            Align2::RIGHT_CENTER,
            format!("#{}", node.id),
            FontId::proportional(10.0 * self.zoom),
            MUTED,
        );

        if node.is_department {
            let child_count = self
                .project
                .nodes
                .iter()
                .filter(|child| child.parent_id.as_deref() == Some(node.id.as_str()))
                .count();
            painter.text(
                Pos2::new(
                    rect.left() + 16.0 * self.zoom,
                    rect.top() + 75.0 * self.zoom,
                ),
                Align2::LEFT_CENTER,
                "ОТДЕЛ · ПОДГРАФ",
                FontId::proportional(10.0 * self.zoom),
                MUTED,
            );
            painter.text(
                Pos2::new(
                    rect.left() + 16.0 * self.zoom,
                    rect.top() + 101.0 * self.zoom,
                ),
                Align2::LEFT_CENTER,
                format!("{child_count} блоков · открыть в инспекторе →"),
                FontId::proportional(10.0 * self.zoom),
                MUTED,
            );
            return;
        }

        let mut y = rect.top() + 50.0 * self.zoom;
        self.draw_section(painter, rect, &mut y, node_index, true, anchors);
        y += 7.0 * self.zoom;
        self.draw_section(painter, rect, &mut y, node_index, false, anchors);
    }

    fn draw_section(
        &self,
        painter: &egui::Painter,
        card: Rect,
        y: &mut f32,
        node_index: usize,
        input: bool,
        anchors: &mut Vec<(Rect, usize, usize, bool)>,
    ) {
        let node = &self.project.nodes[node_index];
        let metrics = if input { &node.inputs } else { &node.outputs };
        let section_height = (92.0 + metrics.len().saturating_sub(1) as f32 * 38.0) * self.zoom;
        let section = Rect::from_min_size(
            Pos2::new(card.left() + 9.0 * self.zoom, *y),
            Vec2::new(card.width() - 18.0 * self.zoom, section_height),
        );
        painter.rect_filled(section, 7.0, PANEL);
        painter.rect_stroke(
            section,
            7.0,
            Stroke::new(1.0 * self.zoom, BORDER),
            egui::StrokeKind::Inside,
        );
        let heading = if input { "ЗАПРОС" } else { "ВЫХОД" };
        let heading_color =
            if input {
                if metrics.iter().enumerate().any(|(index, _)| {
                    self.input_status_for(node_index, index) == MetricStatus::Waiting
                }) {
                    RED
                } else {
                    MUTED
                }
            } else {
                MUTED
            };
        painter.circle_filled(
            Pos2::new(
                section.left() + 11.0 * self.zoom,
                section.top() + 14.0 * self.zoom,
            ),
            3.0 * self.zoom,
            heading_color,
        );
        painter.text(
            Pos2::new(
                section.left() + 20.0 * self.zoom,
                section.top() + 14.0 * self.zoom,
            ),
            Align2::LEFT_CENTER,
            heading,
            FontId::proportional(11.0 * self.zoom),
            heading_color,
        );
        painter.text(
            Pos2::new(
                section.left() + 71.0 * self.zoom,
                section.top() + 14.0 * self.zoom,
            ),
            Align2::LEFT_CENTER,
            metrics.len().to_string(),
            FontId::proportional(10.0 * self.zoom),
            MUTED,
        );
        let attention_count = metrics
            .iter()
            .enumerate()
            .filter(|(index, _)| {
                let status = if input {
                    self.input_status_for(node_index, *index)
                } else {
                    self.output_status_for(node_index, *index)
                };
                if input {
                    status == MetricStatus::Waiting
                } else {
                    matches!(status, MetricStatus::Waiting | MetricStatus::Surplus)
                }
            })
            .count();
        if attention_count > 0 {
            painter.text(
                Pos2::new(
                    section.right() - 42.0 * self.zoom,
                    section.top() + 14.0 * self.zoom,
                ),
                Align2::RIGHT_CENTER,
                format!(
                    "{} {}",
                    attention_count,
                    if input {
                        "ОЖИДАНИЕ"
                    } else {
                        "ВНИМАНИЕ"
                    }
                ),
                FontId::proportional(8.0 * self.zoom),
                RED,
            );
        }
        painter.rect_filled(
            Rect::from_center_size(
                Pos2::new(
                    section.right() - 13.0 * self.zoom,
                    section.top() + 14.0 * self.zoom,
                ),
                Vec2::splat(18.0 * self.zoom),
            ),
            4.0,
            INSET,
        );
        painter.text(
            Pos2::new(
                section.right() - 13.0 * self.zoom,
                section.top() + 14.0 * self.zoom,
            ),
            Align2::CENTER_CENTER,
            "+",
            FontId::proportional(14.0 * self.zoom),
            MUTED,
        );
        for (index, metric) in metrics.iter().enumerate() {
            let row = Rect::from_min_size(
                Pos2::new(
                    section.left() + 7.0 * self.zoom,
                    section.top() + 29.0 * self.zoom + index as f32 * 38.0 * self.zoom,
                ),
                Vec2::new(section.width() - 14.0 * self.zoom, 25.0 * self.zoom),
            );
            painter.rect_filled(row, 5.0, INSET);
            let status = if input {
                self.input_status_for(node_index, index)
            } else {
                self.output_status_for(node_index, index)
            };
            let color = status_color(status);
            painter.circle_filled(
                Pos2::new(row.left() + 9.0 * self.zoom, row.center().y),
                2.7 * self.zoom,
                color,
            );
            let badge = match status {
                MetricStatus::Waiting => "ОЖИДАНИЕ",
                MetricStatus::Satisfied => "УДОВЛЕТВОРЁН",
                MetricStatus::InProgress => "В ПРОЦЕССЕ",
                MetricStatus::Supplied => "ОБЕСПЕЧЕН",
                MetricStatus::Surplus => "ИЗБЫТОК",
            };
            painter.text(
                Pos2::new(row.left() + 17.0 * self.zoom, row.center().y),
                Align2::LEFT_CENTER,
                metric.name.clone(),
                FontId::proportional(11.0 * self.zoom),
                TEXT,
            );
            painter.text(
                Pos2::new(row.right() - 7.0 * self.zoom, row.center().y),
                Align2::RIGHT_CENTER,
                format!("{} {}", format_number(metric.value), metric.unit),
                FontId::proportional(10.0 * self.zoom),
                MUTED,
            );
            let (current, target) = if input {
                (
                    if self.input_has_connections[node_index][index] {
                        self.input_supply[node_index][index]
                    } else {
                        metric.value
                    },
                    metric.required,
                )
            } else {
                (
                    metric.value,
                    if self.output_has_connections[node_index][index] {
                        self.output_demand[node_index][index]
                    } else {
                        metric.required.max(0.0)
                    },
                )
            };
            let progress = if target > 0.0 {
                (current / target).clamp(0.0, 1.0) as f32
            } else if current > 0.0 {
                1.0
            } else {
                0.0
            };
            let progress_start =
                Pos2::new(row.left() + 7.0 * self.zoom, row.bottom() - 2.0 * self.zoom);
            let progress_end = Pos2::new(row.right() - 7.0 * self.zoom, progress_start.y);
            painter.line_segment(
                [progress_start, progress_end],
                Stroke::new(1.5 * self.zoom, BORDER_ACTIVE),
            );
            if progress > 0.0 {
                painter.line_segment(
                    [progress_start, progress_start.lerp(progress_end, progress)],
                    Stroke::new(1.5 * self.zoom, color),
                );
            }
            let badge_rect = Rect::from_min_size(
                Pos2::new(row.left(), row.bottom() + 2.0 * self.zoom),
                Vec2::new(row.width(), 15.0 * self.zoom),
            );
            if self.zoom >= 0.75 {
                painter.text(
                    Pos2::new(badge_rect.left() + 8.0 * self.zoom, badge_rect.center().y),
                    Align2::LEFT_CENTER,
                    format!("{} · #{}", badge, metric.id),
                    FontId::proportional(8.5 * self.zoom),
                    color,
                );
            }
            anchors.push((row.expand(3.0), node_index, index, input));
        }
        let placeholder = Rect::from_min_max(
            Pos2::new(
                section.left() + 7.0 * self.zoom,
                section.bottom() - 18.0 * self.zoom,
            ),
            Pos2::new(
                section.right() - 7.0 * self.zoom,
                section.bottom() - 5.0 * self.zoom,
            ),
        );
        painter.rect_filled(placeholder, 4.0, INSET);
        painter.text(
            placeholder.center(),
            Align2::CENTER_CENTER,
            "+ Добавить показатель",
            FontId::proportional(9.0 * self.zoom),
            MUTED,
        );
        *y += section_height;
    }
}

impl eframe::App for PolygramApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.reference_logo.is_none() {
            self.reference_logo = Some(load_embedded_texture(
                ctx,
                "reference-logo",
                include_bytes!("../assets/reference-logo.png"),
            ));
        }
        if self.reference_icons.is_none() {
            self.reference_icons = Some(load_embedded_texture(
                ctx,
                "reference-dock-icons",
                include_bytes!("../assets/reference-dock-icons.png"),
            ));
        }
        if !self.theme_applied {
            apply_visual_theme(ctx);
            self.theme_applied = true;
        }
        let frame_started = Instant::now();
        if Instant::now() >= self.next_extension_scan {
            self.refresh_extensions();
        }
        let window_focused = ctx.input(|input| input.focused);
        if self.window_focused != window_focused {
            self.window_focused = window_focused;
            self.last_frame_at = None;
            self.frame_samples.clear();
        }
        let frame_time = ctx.input(|input| input.stable_dt);
        let measured_interval = self
            .last_frame_at
            .replace(frame_started)
            .map(|previous| frame_started.duration_since(previous).as_secs_f32());
        self.apply_pending_ticks();
        self.update_stress_test(frame_time);
        self.show_toolbar(ctx);
        self.show_left_dock(ctx);
        self.show_inspector(ctx);
        self.show_canvas(ctx);
        self.show_metric_dialog(ctx);
        self.show_qa(ctx);
        let frame_sample = FrameSample {
            frame_ms: measured_interval.unwrap_or(frame_time) * 1000.0,
            update_ms: frame_started.elapsed().as_secs_f32() * 1000.0,
        };
        if self.window_focused {
            if self.frame_samples.len() == 300 {
                self.frame_samples.pop_front();
            }
            self.frame_samples.push_back(frame_sample);
        }
        let interval = 1.0 / self.frame_cap as f32;
        let wake_early = 0.003 + frame_sample.update_ms / 1000.0;
        ctx.request_repaint_after(std::time::Duration::from_secs_f32(
            (interval - wake_early).max(0.0),
        ));
        if ctx.input(|input| input.key_pressed(Key::Escape)) {
            self.connecting_from = None;
            self.dragging_node = None;
            if self.active_tool == CanvasTool::Connect {
                self.notice = "Создание связи отменено".to_owned();
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MetricStatus {
    Waiting,
    Satisfied,
    InProgress,
    Supplied,
    Surplus,
}

fn input_status(metric: &Metric) -> MetricStatus {
    if metric.value >= metric.required {
        MetricStatus::Satisfied
    } else {
        MetricStatus::Waiting
    }
}

fn update_dynamic_metrics(project: &mut Project, elapsed: f64) -> usize {
    let mut changed = 0;
    for (node_index, node) in project.nodes.iter_mut().enumerate() {
        for (metric_index, metric) in node.inputs.iter_mut().enumerate() {
            let phase = elapsed * 1.2 + node_index as f64 * 0.73 + metric_index as f64 * 1.91;
            let wave = (phase.sin() + 1.0) * 0.5;
            let value = metric.required.max(0.0) * (0.2 + wave * 1.6);
            if (metric.value - value).abs() > 0.001 {
                metric.value = value;
                changed += 1;
            }
        }
        for (metric_index, metric) in node.outputs.iter_mut().enumerate() {
            let phase = elapsed * 1.2 + node_index as f64 * 0.73 + metric_index as f64 * 1.91 + 2.4;
            let wave = (phase.sin() + 1.0) * 0.5;
            let value = metric.required.max(0.0) * (0.2 + wave * 1.6);
            if (metric.value - value).abs() > 0.001 {
                metric.value = value;
                changed += 1;
            }
        }
    }
    changed
}

fn input_status_from_flow(metric: &Metric, connected: bool, supplied: f64) -> MetricStatus {
    if !connected {
        return input_status(metric);
    }
    if supplied + 0.0001 >= metric.required {
        MetricStatus::Satisfied
    } else if supplied > 0.0001 {
        MetricStatus::InProgress
    } else {
        MetricStatus::Waiting
    }
}

fn output_status_from_flow(
    metric: &Metric,
    connected: bool,
    demand: f64,
    standalone_limit: Option<f64>,
) -> MetricStatus {
    if !connected {
        return output_status_with_limit(metric, standalone_limit);
    }
    if metric.value + 0.0001 < demand {
        MetricStatus::Waiting
    } else if metric.value > demand + 0.0001 {
        MetricStatus::Surplus
    } else {
        MetricStatus::Supplied
    }
}

fn output_status_with_limit(metric: &Metric, limit: Option<f64>) -> MetricStatus {
    if limit.is_some_and(|max| metric.value > max) {
        MetricStatus::Surplus
    } else if metric.required > 0.0 && metric.value >= metric.required {
        MetricStatus::Supplied
    } else if metric.value > 0.0 {
        MetricStatus::InProgress
    } else {
        MetricStatus::Waiting
    }
}

fn effective_output_limit(
    own_limit: Option<f64>,
    target_count: usize,
    all_targets_limited: bool,
    target_limit_sum: f64,
) -> Option<f64> {
    if target_count > 0 && all_targets_limited {
        Some(target_limit_sum)
    } else {
        own_limit
    }
}

fn status_color(status: MetricStatus) -> Color32 {
    match status {
        MetricStatus::Waiting => AMBER,
        MetricStatus::Satisfied | MetricStatus::Supplied => GREEN,
        MetricStatus::InProgress => BLUE,
        MetricStatus::Surplus => PURPLE,
    }
}

fn metric_status_detail(status: MetricStatus, current: f64, target: f64, unit: &str) -> String {
    let message = match status {
        MetricStatus::Waiting => "Ожидание: текущего потока недостаточно",
        MetricStatus::Satisfied => "Требование выполнено",
        MetricStatus::InProgress => "Поступление идёт, цель ещё не достигнута",
        MetricStatus::Supplied => "Спрос обеспечен",
        MetricStatus::Surplus => "Значение выше спроса или заданного лимита",
    };
    format!(
        "{message} · {} / {} {}",
        format_number(current),
        format_number(target),
        unit
    )
}

fn metric_y(node: &Node, input: bool, index: usize) -> f32 {
    let top = node.y + 50.0;
    let section_top = if input {
        top
    } else {
        top + 92.0 + node.inputs.len().saturating_sub(1) as f32 * 38.0 + 7.0
    };
    section_top + 29.0 + index as f32 * 38.0 + 12.5
}

fn distance_to_segment(point: Pos2, start: Pos2, end: Pos2) -> f32 {
    let segment = end - start;
    let length_squared = segment.length_sq();
    if length_squared <= f32::EPSILON {
        return point.distance(start);
    }
    let t = ((point - start).dot(segment) / length_squared).clamp(0.0, 1.0);
    point.distance(start + segment * t)
}

fn cubic_bezier(start: Pos2, c1: Pos2, c2: Pos2, end: Pos2, t: f32) -> Pos2 {
    let inverse = 1.0 - t;
    (start.to_vec2() * inverse.powi(3)
        + c1.to_vec2() * (3.0 * inverse.powi(2) * t)
        + c2.to_vec2() * (3.0 * inverse * t.powi(2))
        + end.to_vec2() * t.powi(3))
    .to_pos2()
}

fn format_number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_status_uses_sum_of_target_limits_when_configured() {
        let project = Project::default();
        let source = &project.nodes[0].outputs[0];
        let effective_limit = effective_output_limit(Some(30_000.0), 2, true, 20_000.0);
        assert_eq!(
            output_status_with_limit(source, effective_limit),
            MetricStatus::Surplus
        );
        assert_eq!(
            effective_output_limit(Some(30_000.0), 2, false, 20_000.0),
            Some(30_000.0)
        );
    }

    #[test]
    fn benchmark_graph_has_one_thousand_nodes_and_two_thousand_edges() {
        let network = Network {
            ticks: std::sync::Arc::new(crossbeam_queue::ArrayQueue::new(1)),
            polls: std::sync::Arc::new(std::sync::RwLock::new(Vec::new())),
        };
        let mut app =
            PolygramApp::new(network, "test-adapter".to_owned()).expect("local store opens");
        app.start_stress_test();

        assert_eq!(app.project.nodes.len(), 1_000);
        assert_eq!(app.project.edges.len(), 2_000);
        assert_eq!(app.resolved_edges.len(), 2_000);
        assert_eq!(app.node_indices.len(), 1_000);
        assert!(app.stress_running);
        assert_eq!(app.stress_speed, 10.0);
        assert!(app.project.validate().is_ok());
    }

    #[test]
    fn dynamic_metrics_change_values_and_unconnected_status_over_time() {
        let network = Network {
            ticks: std::sync::Arc::new(crossbeam_queue::ArrayQueue::new(1)),
            polls: std::sync::Arc::new(std::sync::RwLock::new(Vec::new())),
        };
        let mut app =
            PolygramApp::new(network, "test-adapter".to_owned()).expect("local store opens");
        app.generate_sample(3);

        update_dynamic_metrics(&mut app.project, 0.0);
        app.refresh_edge_flows();
        let initial_value = app.project.nodes[0].inputs[0].value;
        let initial_status = app.input_status_for(0, 0);

        let trough_time = 3.0 * std::f64::consts::FRAC_PI_2 / 1.2;
        let changed = update_dynamic_metrics(&mut app.project, trough_time);
        app.refresh_edge_flows();

        assert_eq!(changed, 6);
        assert_ne!(app.project.nodes[0].inputs[0].value, initial_value);
        assert_ne!(app.input_status_for(0, 0), initial_status);
        assert_eq!(app.input_status_for(0, 0), MetricStatus::Waiting);
    }

    #[test]
    fn dynamic_output_values_change_connected_input_status() {
        let network = Network {
            ticks: std::sync::Arc::new(crossbeam_queue::ArrayQueue::new(1)),
            polls: std::sync::Arc::new(std::sync::RwLock::new(Vec::new())),
        };
        let mut app =
            PolygramApp::new(network, "test-adapter".to_owned()).expect("local store opens");
        app.generate_sample(2);

        update_dynamic_metrics(&mut app.project, 0.0);
        app.refresh_edge_flows();
        assert_eq!(app.input_status_for(1, 0), MetricStatus::Satisfied);

        let output_trough = (3.0 * std::f64::consts::FRAC_PI_2 - 2.4) / 1.2;
        update_dynamic_metrics(&mut app.project, output_trough);
        app.refresh_edge_flows();

        assert_eq!(app.input_status_for(1, 0), MetricStatus::InProgress);
    }

    #[test]
    fn fit_graph_places_the_default_graph_inside_the_canvas() {
        let network = Network {
            ticks: std::sync::Arc::new(crossbeam_queue::ArrayQueue::new(1)),
            polls: std::sync::Arc::new(std::sync::RwLock::new(Vec::new())),
        };
        let mut app =
            PolygramApp::new(network, "test-adapter".to_owned()).expect("local store opens");
        let canvas = Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 800.0));
        app.fit_graph(canvas);

        assert!(app.zoom > 0.1);
        for node in &app.project.nodes {
            assert!(canvas.contains(app.node_screen_rect(canvas, node).min));
            assert!(canvas.contains(app.node_screen_rect(canvas, node).max));
        }
    }

    #[test]
    fn connected_supply_is_shared_proportionally_and_drives_metric_status() {
        let network = Network {
            ticks: std::sync::Arc::new(crossbeam_queue::ArrayQueue::new(1)),
            polls: std::sync::Arc::new(std::sync::RwLock::new(Vec::new())),
        };
        let mut app =
            PolygramApp::new(network, "test-adapter".to_owned()).expect("local store opens");
        app.project.nodes[0].outputs[0].value = 150.0;
        app.project.nodes[1].inputs[0].required = 100.0;
        let third_node = Node {
            id: "PROC_TEST_03".to_owned(),
            name: "Проверка".to_owned(),
            kind: "Процесс".to_owned(),
            parent_id: None,
            is_department: false,
            is_portal: false,
            x: 600.0,
            y: 200.0,
            inputs: vec![Metric {
                id: "M_TEST_INPUT".to_owned(),
                name: "Лиды".to_owned(),
                value: 0.0,
                required: 200.0,
                max: None,
                unit: "ед.".to_owned(),
            }],
            outputs: Vec::new(),
            poll_url: String::new(),
            poll_interval_secs: 5,
        };
        app.project.nodes.push(third_node);
        app.project.edges.push(Edge {
            from_node: "PROC_MKT_01".to_owned(),
            from_metric: "M_LEADS".to_owned(),
            to_node: "PROC_TEST_03".to_owned(),
            to_metric: "M_TEST_INPUT".to_owned(),
        });
        app.rebuild_spatial();

        assert!((app.edge_allocations[0] - 50.0).abs() < 0.0001);
        assert!((app.edge_allocations[1] - 100.0).abs() < 0.0001);
        assert_eq!(app.input_status_for(1, 0), MetricStatus::InProgress);
        assert_eq!(app.output_status_for(0, 0), MetricStatus::Waiting);

        app.project.nodes[0].outputs[0].value = 350.0;
        app.refresh_edge_flows();
        assert_eq!(app.input_status_for(1, 0), MetricStatus::Satisfied);
        assert_eq!(app.output_status_for(0, 0), MetricStatus::Surplus);
    }

    #[test]
    fn portal_transfers_received_supply_to_another_department() {
        let network = Network {
            ticks: std::sync::Arc::new(crossbeam_queue::ArrayQueue::new(1)),
            polls: std::sync::Arc::new(std::sync::RwLock::new(Vec::new())),
        };
        let mut app =
            PolygramApp::new(network, "test-adapter".to_owned()).expect("local store opens");
        app.project.nodes[1].is_portal = true;
        app.project.nodes[1].kind = "Портал".to_owned();
        app.project.nodes[1].parent_id = Some("DEPT_01".to_owned());
        app.project.edges.clear();
        let department = Node {
            id: "DEPT_01".to_owned(),
            name: "Продажи".to_owned(),
            kind: "Отдел".to_owned(),
            parent_id: None,
            is_department: true,
            is_portal: false,
            x: 500.0,
            y: 100.0,
            inputs: Vec::new(),
            outputs: Vec::new(),
            poll_url: String::new(),
            poll_interval_secs: 5,
        };
        let child = Node {
            id: "PROC_CHILD".to_owned(),
            name: "Закрытие сделки".to_owned(),
            kind: "Процесс".to_owned(),
            parent_id: Some("DEPT_01".to_owned()),
            is_department: false,
            is_portal: false,
            x: 900.0,
            y: 100.0,
            inputs: vec![Metric {
                id: "M_CHILD_LEADS".to_owned(),
                name: "Лиды".to_owned(),
                value: 0.0,
                required: 18_000.0,
                max: None,
                unit: "ед.".to_owned(),
            }],
            outputs: Vec::new(),
            poll_url: String::new(),
            poll_interval_secs: 5,
        };
        app.project.nodes.extend([department, child]);
        app.project.edges.extend([
            Edge {
                from_node: "PROC_MKT_01".to_owned(),
                from_metric: "M_LEADS".to_owned(),
                to_node: "PROC_SALES_01".to_owned(),
                to_metric: "M_SALES_LEADS".to_owned(),
            },
            Edge {
                from_node: "PROC_SALES_01".to_owned(),
                from_metric: "M_REVENUE".to_owned(),
                to_node: "PROC_CHILD".to_owned(),
                to_metric: "M_CHILD_LEADS".to_owned(),
            },
        ]);
        assert!(app.project.validate().is_ok());
        app.rebuild_render_indexes();

        assert_eq!(app.project.nodes[1].outputs[0].value, 18_000.0);
        assert_eq!(app.input_supply[3][0], 18_000.0);
    }

    #[test]
    fn adding_metrics_skips_ids_already_used_in_the_project() {
        let network = Network {
            ticks: std::sync::Arc::new(crossbeam_queue::ArrayQueue::new(1)),
            polls: std::sync::Arc::new(std::sync::RwLock::new(Vec::new())),
        };
        let mut app =
            PolygramApp::new(network, "test-adapter".to_owned()).expect("local store opens");

        app.project.nodes[1].inputs[0].id = "PROC_MKT_01_OUT_01".to_owned();
        assert_eq!(
            app.next_metric_id("PROC_MKT_01", MetricDirection::Output),
            "PROC_MKT_01_OUT_02"
        );
        assert_eq!(
            app.next_metric_id("PROC_MKT_01", MetricDirection::Input),
            "PROC_MKT_01_IN_01"
        );
    }

    #[test]
    fn card_metric_placeholders_open_the_matching_metric_form() {
        let network = Network {
            ticks: std::sync::Arc::new(crossbeam_queue::ArrayQueue::new(1)),
            polls: std::sync::Arc::new(std::sync::RwLock::new(Vec::new())),
        };
        let app = PolygramApp::new(network, "test-adapter".to_owned()).expect("local store opens");
        let canvas = Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 900.0));

        assert!(matches!(
            app.metric_add_target_at(canvas, Pos2::new(250.0, 250.0), &[0]),
            Some((0, MetricDirection::Input))
        ));
        assert!(matches!(
            app.metric_add_target_at(canvas, Pos2::new(250.0, 350.0), &[0]),
            Some((0, MetricDirection::Output))
        ));
    }

    #[test]
    fn zoom_animation_keeps_the_world_point_under_the_cursor() {
        let network = Network {
            ticks: std::sync::Arc::new(crossbeam_queue::ArrayQueue::new(1)),
            polls: std::sync::Arc::new(std::sync::RwLock::new(Vec::new())),
        };
        let mut app =
            PolygramApp::new(network, "test-adapter".to_owned()).expect("local store opens");
        let canvas = Rect::from_min_size(Pos2::new(100.0, 50.0), Vec2::new(1200.0, 900.0));
        let screen_anchor = Vec2::new(400.0, 300.0);
        let world_anchor = Pos2::new(300.0, 200.0);
        app.zoom_target = 2.0;
        app.zoom_anchor = Some((screen_anchor, world_anchor));

        app.advance_zoom(1.0 / 60.0);

        assert!(app.zoom > 1.0 && app.zoom < app.zoom_target);
        assert!(
            app.world_to_screen(canvas, world_anchor)
                .distance(canvas.min + screen_anchor)
                < 0.001
        );
    }

    #[test]
    fn default_frame_cap_does_not_limit_high_refresh_displays_to_sixty_fps() {
        let network = Network {
            ticks: std::sync::Arc::new(crossbeam_queue::ArrayQueue::new(1)),
            polls: std::sync::Arc::new(std::sync::RwLock::new(Vec::new())),
        };
        let app = PolygramApp::new(network, "test-adapter".to_owned()).expect("local store opens");

        assert_eq!(app.frame_cap, 144);
    }
}

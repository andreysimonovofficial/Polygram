use eframe::egui;
use polygram_api::ApiService;
use polygram_canvas::Canvas;
use polygram_core::{runtime::{EngineHandle, EngineRuntime}, Command, Direction, MetricKind};

pub struct PolygramApp {
    pub runtime: EngineHandle,
    /// Владение потоком движка (только для `new()`; в `new_with_api` состояние делится через API).
    #[allow(dead_code)]
    pub _engine: Option<EngineRuntime>,
    pub canvas: Canvas,
    pub show_command_palette: bool,
    pub command_input: String,
    pub last_status: String,
    pub right_click: Option<(egui::Pos2, Option<String>)>,
    pub command_log: Vec<CommandLogEntry>,
    pub rename_target: Option<String>,
    pub rename_input: String,
    /// Инлайн-редактирование названия объекта прямо на холсте (Figma-стиль).
    pub edit_target: Option<String>,
    pub edit_input: String,
    /// Модальное диалоговое окно добавления показателя.
    pub add_metric_target: Option<(String, Direction)>,
    pub add_metric_name: String,
    pub add_metric_kind: MetricKind,
    pub add_metric_value: String,
    pub add_metric_unit: String,
    /// Модальное диалоговое окно редактирования списка показателей контейнера.
    pub container_menu_target: Option<(String, Direction)>,
    /// Временные буферы для редактирования значений и единиц показателей в диалоге контейнера.
    pub container_menu_edits: Vec<(String, String, String)>,
    pub drag_object: Option<String>,
    pub drag_start_screen: Option<egui::Pos2>,
    pub drag_start_cell: Option<(i32, i32)>,
    /// Количество кадров для прогрева кэша галлей/глифов при старте,
    /// чтобы первое панорамирование было гладким без «холодного» старта.
    pub warmup_frames: u32,
}

pub struct CommandLogEntry {
    pub cmd: String,
    pub result: String,
}

impl PolygramApp {
    pub fn new() -> Self {
        let engine = polygram_core::Engine::new();
        let runtime = EngineRuntime::spawn(engine);
        let handle = runtime.handle();
        let canvas = Canvas::new();
        Self {
            runtime: handle,
            _engine: Some(runtime),
            canvas,
            show_command_palette: false,
            command_input: String::new(),
            last_status: "Готов".to_owned(),
            right_click: None,
            command_log: Vec::new(),
            rename_target: None,
            rename_input: String::new(),
            edit_target: None,
            edit_input: String::new(),
            add_metric_target: None,
            add_metric_name: String::new(),
            add_metric_kind: MetricKind::Number,
            add_metric_value: String::new(),
            add_metric_unit: String::new(),
            container_menu_target: None,
            container_menu_edits: Vec::new(),
            drag_object: None,
            drag_start_screen: None,
            drag_start_cell: None,
            warmup_frames: 120,
        }
    }

    /// Создать приложение, разделяя состояние с уже существующим ApiService
    /// (и его MCP-потоком).GUI и MCP делят один EngineHandle.
    pub fn new_with_api(api: ApiService) -> Self {
        let runtime = api.handle();
        let canvas = Canvas::new();
        Self {
            runtime,
            _engine: None,
            canvas,
            show_command_palette: false,
            command_input: String::new(),
            last_status: "Готов (MCP подключён)".to_owned(),
            right_click: None,
            command_log: Vec::new(),
            rename_target: None,
            rename_input: String::new(),
            edit_target: None,
            edit_input: String::new(),
            add_metric_target: None,
            add_metric_name: String::new(),
            add_metric_kind: MetricKind::Number,
            add_metric_value: String::new(),
            add_metric_unit: String::new(),
            container_menu_target: None,
            container_menu_edits: Vec::new(),
            drag_object: None,
            drag_start_screen: None,
            drag_start_cell: None,
            warmup_frames: 120,
        }
    }

    pub fn apply_cmd(&mut self, cmd: Command) {
        let cmd_str = serde_json::to_string(&cmd).unwrap_or_default();
        match self.runtime.apply(cmd) {
            Ok(applied) => {
                self.last_status = format!("Ревизия {}", applied.rev);
                self.command_log.push(CommandLogEntry {
                    cmd: cmd_str,
                    result: format!("OK ревизия {}", applied.rev),
                });
            }
            Err(e) => {
                self.last_status = format!("Ошибка: {e:?}");
                self.command_log.push(CommandLogEntry {
                    cmd: cmd_str,
                    result: format!("Ошибка: {e:?}"),
                });
            }
        }
        if self.command_log.len() > 200 {
            self.command_log.remove(0);
        }
    }
}

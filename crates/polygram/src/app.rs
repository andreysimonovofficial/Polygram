use eframe::egui;
use polygram_canvas::Canvas;
use polygram_core::{runtime::EngineRuntime, Command};

pub struct PolygramApp {
    pub runtime: EngineRuntime,
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
        let canvas = Canvas::new();
        Self {
            runtime,
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
            drag_object: None,
            drag_start_screen: None,
            drag_start_cell: None,
            warmup_frames: 120,
        }
    }

    pub fn apply_cmd(&mut self, cmd: Command) {
        let cmd_str = serde_json::to_string(&cmd).unwrap_or_default();
        match self.runtime.handle().apply(cmd) {
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

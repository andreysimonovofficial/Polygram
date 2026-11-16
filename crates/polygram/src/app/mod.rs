use egui::Color32;
use egui::Vec2;
use polygram_canvas::{Canvas, GridMode};
use polygram_core::Published;
use reqwest::blocking::Client;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

pub mod binding;
pub mod bindings;
pub mod chat;
pub mod cmd;
pub mod commands;
pub mod connection;
pub mod connectors;
pub mod departments;
pub mod dialogs;
pub mod drag;
pub mod fonts;
pub mod process;
pub mod settings;
pub mod snapshot;
pub mod state;
pub mod ui_config;

use bindings::BindingManager;
use chat::ChatState;
use commands::CommandState;
use connection::ConnectionState;
use connectors::ConnectorManager;
use dialogs::DialogState;
use drag::DragState;
use std::path::PathBuf;
use std::sync::Mutex;
use ui_config::UiConfig;

pub use state::CommandLogEntry;

/// Инъекция виртуального ввода (мышь + клавиатура) в egui через синтетические события.
/// Вызывается из PolygramApp::logic() после poll_snapshot().
/// Курсор плавно перемещается по кривой Безье к цели, поддерживает рисование фигур.
/// Вызывается из PolygramApp::logic() после poll_snapshot().
impl PolygramApp {
    pub fn inject_virtual_input(&mut self, ctx: &egui::Context) {
        let Some(published) = &self.connection.snapshot else {
            return;
        };
        let vi = &published.virtual_input;
        let prev = &self.virtual_input;

        // Целевая позиция курсора (из движка).
        let target = egui::Pos2::new(vi.target_x, vi.target_y);

        // Сброс индекса пути и мгновенное перемещение в первую точку при получении нового пути.
        if vi.path_points != prev.path_points && !vi.path_points.is_empty() {
            self.virtual_path_index = 0;
            self.virtual_cursor_pos = egui::Pos2::new(
                vi.path_points[0].0,
                vi.path_points[0].1,
            );
        }

        // Курсор виден когда есть цель, зажата кнопка или идёт путь по точкам.
        let cursor_visible = target.x != 0.0
            || target.y != 0.0
            || vi.mouse_left_pressed
            || vi.mouse_right_pressed
            || (!vi.path_points.is_empty() && self.virtual_path_index < vi.path_points.len());

        if !cursor_visible {
            // Курсор скрыт — сбрасываем позицию и не перерисовываем.
            if self.virtual_cursor_pos != egui::Pos2::default() {
                self.virtual_cursor_pos = egui::Pos2::default();
                ctx.request_repaint();
            }
            return;
        }

        let pos = if !vi.path_points.is_empty() && self.virtual_path_index < vi.path_points.len() {
            // Движение по точкам пути (линейная интерполяция между точками).
            let seg_target = egui::Pos2::new(
                vi.path_points[self.virtual_path_index].0,
                vi.path_points[self.virtual_path_index].1,
            );
            let lerp_factor = 0.15;
            let dx = (seg_target.x - self.virtual_cursor_pos.x).clamp(-200.0, 200.0);
            let dy = (seg_target.y - self.virtual_cursor_pos.y).clamp(-200.0, 200.0);
            let new_pos = egui::Pos2::new(
                self.virtual_cursor_pos.x + dx * lerp_factor,
                self.virtual_cursor_pos.y + dy * lerp_factor,
            );
            let reached = (new_pos.x - seg_target.x).abs() < 1.0
                && (new_pos.y - seg_target.y).abs() < 1.0;
            if reached {
                self.virtual_path_index += 1;
            }
            new_pos
        } else {
            // Простое плавное движение к цели.
            let lerp_factor = 0.15;
            let dx = (target.x - self.virtual_cursor_pos.x).clamp(-200.0, 200.0);
            let dy = (target.y - self.virtual_cursor_pos.y).clamp(-200.0, 200.0);
            egui::Pos2::new(
                self.virtual_cursor_pos.x + dx * lerp_factor,
                self.virtual_cursor_pos.y + dy * lerp_factor,
            )
        };

        self.virtual_cursor_pos = pos;

        // Запоминаем точку начала рисования при нажатии кнопки.
        if vi.mouse_left_pressed && !prev.mouse_left_pressed {
            self.virtual_draw_start = Some(pos);
        } else if !vi.mouse_left_pressed && prev.mouse_left_pressed {
            self.virtual_draw_start = None;
        }

        let mut events: Vec<egui::Event> = Vec::new();

        // Перемещение курсора — событие только если позиция изменилась заметно.
        if (pos.x - prev.mouse_x).abs() > 0.5 || (pos.y - prev.mouse_y).abs() > 0.5 {
            events.push(egui::Event::PointerMoved(pos));
        }

        // Левая кнопка мыши.
        if vi.mouse_left_pressed && !prev.mouse_left_pressed {
            events.push(egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            });
        } else if !vi.mouse_left_pressed && prev.mouse_left_pressed {
            events.push(egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            });
        }

        // Правая кнопка мыши.
        if vi.mouse_right_pressed && !prev.mouse_right_pressed {
            events.push(egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Secondary,
                pressed: true,
                modifiers: Default::default(),
            });
        } else if !vi.mouse_right_pressed && prev.mouse_right_pressed {
            events.push(egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Secondary,
                pressed: false,
                modifiers: Default::default(),
            });
        }

        // Текст виртуальной клавиатуры — отправляем при изменении непустого текста.
        if !vi.keyboard_text.is_empty() && vi.keyboard_text != prev.keyboard_text {
            events.push(egui::Event::Text(vi.keyboard_text.clone()));
        }

        if !events.is_empty() {
            ctx.input_mut(|input| {
                input.events.extend(events);
            });
        }

        // Сохраняем обработанное состояние.
        let mut next = vi.clone();
        next.mouse_x = pos.x;
        next.mouse_y = pos.y;
        if next != *prev {
            self.virtual_input = next;
            ctx.request_repaint();
        }

        // Непрерывная перерисовка, пока курсор виден.
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}

pub struct PolygramApp {
    pub canvas: Canvas,
    pub connection: ConnectionState,
    pub commands: CommandState,
    pub dialogs: DialogState,
    pub drag: DragState,
    pub right_click: Option<(egui::Pos2, Option<String>)>,
    /// Выделенные объекты (ID) — grupповое выделение через Selection Box и Ctrl+клик.
    pub selected_objects: Vec<String>,
    /// Менеджер API-привязок показателей.
    pub bindings: BindingManager,
    pub show_mcp_panel: bool,
    pub show_sidebar: bool,
    pub show_command_log_panel: bool,
    pub show_frontend_log_panel: bool,
    pub show_chat_panel: bool,
    pub show_connectors_panel: bool,
    pub grid_mode: GridMode,
    pub grid_color: [u8; 3],
    pub show_cursor_coords: bool,
    /// Окно настроек темы открыто.
    pub show_settings: bool,
    /// Полноэкранный экран показателей открыт.
    pub show_metrics_screen: bool,
    /// Объект, для которого открыт экран показателей.
    pub metrics_target_object: Option<String>,
    pub last_status: String,
    pub chat: ChatState,
    /// Экспорт/импорт отдела: флаги запроса открытия диалога файла.
    pub export_requested: bool,
    pub import_requested: bool,
    pub export_department_requested: bool,
    pub import_department_requested: bool,
    /// Экспорт/импорт проекта: флаги запроса открытия диалога файла.
    pub open_project_requested: bool,
    pub save_project_requested: bool,
    pub save_project_as_requested: bool,
    /// Менеджер коннекторов: кэш файлов connectors/ и статус подключения.
    pub connectors: ConnectorManager,
    /// Несохранённые изменения (флаг "грязного" состояния).
    pub dirty: bool,
    /// Последний открытый файл проекта.
    pub last_opened_file: Option<String>,
    /// Подтверждение закрытия с несохранёнными изменениями ожидается.
    pub close_confirmation_pending: bool,
    /// Запрос на снимок экрана (устанавливается из меню).
    pub screenshot_requested: bool,
    /// Результат последнего снимка экрана (записывается из колбэка egui).
    pub screenshot_result: Arc<Mutex<Option<Result<PathBuf, String>>>>,
    /// Запрос на вписывание всех объектов в область просмотра камеры (устанавливается из меню "Вид").
    pub fit_camera_requested: bool,
    /// Последняя ревизия, для которой был сделан скриншот.
    pub last_screenshot_rev: u64,
    /// Момент последнего автоматического скриншота (ограничение частоты).
    pub last_screenshot_time: std::time::Instant,
    /// Последнее обработанное состояние виртуального ввода (для детекта изменений и инъекции egui-событий).
    pub virtual_input: polygram_core::VirtualInputState,
    /// Текущая отрисовываемая позиция виртуального курсора (плавно интерполируется к цели).
    pub virtual_cursor_pos: egui::Pos2,
    /// Индекс текущей точки на кривой VirtualMousePath (отслеживается в GUI).
    pub virtual_path_index: usize,
    /// Точка начала рисования фигуры (позиция курсора в момент нажатия кнопки).
    pub virtual_draw_start: Option<egui::Pos2>,

}

impl PolygramApp {
    pub fn new(snapshot_rx: mpsc::Receiver<Arc<Published>>) -> Self {
        let mut canvas = Canvas::new();
        let api_client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        let api_base = "http://127.0.0.1:8080".to_owned();
        let mut chat = ChatState::default();
        chat.chat_rx = ChatState::start_polling(api_client.clone(), api_base.clone());
        let ui_config = UiConfig::load();
        let grid_color = ui_config.grid_color;
        // Применяем сохранённую тему оформления.
        canvas.theme = ui_config.theme.into_theme();
        canvas.theme.grid_color = Color32::from_rgb(grid_color[0], grid_color[1], grid_color[2]);
        // Восстанавливаем последнюю позицию камеры.
        canvas.camera.offset = Vec2::new(ui_config.camera_offset[0], ui_config.camera_offset[1]);
        canvas.camera.zoom = ui_config.camera_zoom;
        Self {
            canvas,
            connection: ConnectionState::new(api_client, api_base, snapshot_rx),
            commands: CommandState::new(),
            dialogs: DialogState::new(),
            drag: DragState::new(),
            right_click: None,
            selected_objects: Vec::new(),
            bindings: BindingManager::new(),
            show_mcp_panel: false,
            show_sidebar: ui_config.show_sidebar,
            show_command_log_panel: ui_config.show_command_log_panel,
            show_frontend_log_panel: ui_config.show_frontend_log_panel,
            show_chat_panel: ui_config.show_chat_panel,
            show_connectors_panel: ui_config.show_connectors_panel,
            grid_mode: ui_config.grid_mode,
            grid_color,
            show_cursor_coords: ui_config.show_cursor_coords,
            show_settings: false,
            show_metrics_screen: false,
            metrics_target_object: None,
            last_status: "API сервер не подключён".to_owned(),
            chat,
            export_requested: false,
            import_requested: false,
            export_department_requested: false,
            import_department_requested: false,
            open_project_requested: false,
            save_project_requested: false,
            save_project_as_requested: false,
            connectors: ConnectorManager::new(),
            dirty: false,
            last_opened_file: ui_config.last_opened_file.clone(),
            close_confirmation_pending: false,
            screenshot_requested: false,
            screenshot_result: Arc::new(Mutex::new(None)),
            fit_camera_requested: false,
            last_screenshot_rev: 0,
            last_screenshot_time: std::time::Instant::now(),
            virtual_input: polygram_core::VirtualInputState::default(),
            virtual_cursor_pos: egui::Pos2::default(),
            virtual_path_index: 0,
            virtual_draw_start: None,
        }
    }
}

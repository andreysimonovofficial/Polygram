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

        // Автоскриншот при движении курсора — чтобы видеть каждое положение.
        let cursor_moved = (pos.x - prev.mouse_x).abs() > 1.0 || (pos.y - prev.mouse_y).abs() > 1.0;
        if cursor_visible && cursor_moved {
            self.screenshot_requested = true;
        }

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

        // Средняя кнопка мыши.
        if vi.mouse_middle_pressed && !prev.mouse_middle_pressed {
            events.push(egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Middle,
                pressed: true,
                modifiers: Default::default(),
            });
        } else if !vi.mouse_middle_pressed && prev.mouse_middle_pressed {
            events.push(egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Middle,
                pressed: false,
                modifiers: Default::default(),
            });
        }

        // Виртуальная клавиатура: нажатие/отпускание клавиш (комбинации).
        let prev_keys: std::collections::HashSet<&str> = prev
            .keyboard_keys
            .iter()
            .map(|s| s.as_str())
            .collect();
        let curr_keys: std::collections::HashSet<&str> = vi
            .keyboard_keys
            .iter()
            .map(|s| s.as_str())
            .collect();
        for key in curr_keys.difference(&prev_keys) {
            if let Some(egui_key) = parse_key_name(key) {
                events.push(egui::Event::Key {
                    key: egui_key,
                    pressed: true,
                    modifiers: Default::default(),
                    repeat: false,
                    physical_key: None,
                });
            }
        }
        for key in prev_keys.difference(&curr_keys) {
            if let Some(egui_key) = parse_key_name(key) {
                events.push(egui::Event::Key {
                    key: egui_key,
                    pressed: false,
                    modifiers: Default::default(),
                    repeat: false,
                    physical_key: None,
                });
            }
        }

        // Виртуальная мышь: прокрутка колёсика.
        if vi.wheel_delta != (0.0, 0.0) {
            events.push(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::Vec2::new(vi.wheel_delta.0, vi.wheel_delta.1),
                modifiers: Default::default(),
                phase: egui::TouchPhase::End,
                source: egui::MouseWheelSource::Trackpad,
            });
        }

        // Виртуальная мышь: перетаскивание объекта.
        if let Some(drag_to) = vi.drag_to {
            if !prev.mouse_left_pressed {
                // Начало перетаскивания: нажатие левой кнопки в текущей позиции.
                events.push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                });
            }
            // Проверяем достижение цели перетаскивания.
            let drag_reached = (pos.x - drag_to.0).abs() < 1.0 && (pos.y - drag_to.1).abs() < 1.0;
            if drag_reached && prev.mouse_left_pressed {
                // Конец перетаскивания: отпускание левой кнопки в целевой позиции.
                events.push(egui::Event::PointerButton {
                    pos: egui::Pos2::new(drag_to.0, drag_to.1),
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: Default::default(),
                });
            }
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

        // Очищаем состояние перетаскивания после завершения.
        if next.drag_to.is_some() && !prev.mouse_left_pressed {
            // Курсор достиг цели и кнопка отпущена — сбрасываем состояние перетаскивания.
            next.drag_target = None;
            next.drag_to = None;
            next.target_x = 0.0;
            next.target_y = 0.0;
        }

        // Сбрасываем дельта колёсика после обработки.
        if next.wheel_delta != (0.0, 0.0) {
            next.wheel_delta = (0.0, 0.0);
        }

        if next != *prev {
            self.virtual_input = next;
            ctx.request_repaint();
        }

        // Непрерывная перерисовка, пока курсор виден.
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}

/// Преобразует строковое имя клавиши в `egui::Key`.
/// Поддерживает буквы, цифры, функциональные клавиши, модификаторы и специальные клавиши.
fn parse_key_name(name: &str) -> Option<egui::Key> {
    let key = match name {
        // Буквы
        "A" => egui::Key::A, "B" => egui::Key::B, "C" => egui::Key::C, "D" => egui::Key::D,
        "E" => egui::Key::E, "F" => egui::Key::F, "G" => egui::Key::G, "H" => egui::Key::H,
        "I" => egui::Key::I, "J" => egui::Key::J, "K" => egui::Key::K, "L" => egui::Key::L,
        "M" => egui::Key::M, "N" => egui::Key::N, "O" => egui::Key::O, "P" => egui::Key::P,
        "Q" => egui::Key::Q, "R" => egui::Key::R, "S" => egui::Key::S, "T" => egui::Key::T,
        "U" => egui::Key::U, "V" => egui::Key::V, "W" => egui::Key::W, "X" => egui::Key::X,
        "Y" => egui::Key::Y, "Z" => egui::Key::Z,
        // Цифры
        "0" => egui::Key::Num0, "1" => egui::Key::Num1, "2" => egui::Key::Num2,
        "3" => egui::Key::Num3, "4" => egui::Key::Num4, "5" => egui::Key::Num5,
        "6" => egui::Key::Num6, "7" => egui::Key::Num7, "8" => egui::Key::Num8,
        "9" => egui::Key::Num9,
        // Модификаторы не являются вариантами egui::Key — они через egui::Modifiers.
        // Специальные клавиши
        "Enter" => egui::Key::Enter, "Return" => egui::Key::Enter,
        "Escape" => egui::Key::Escape, "Esc" => egui::Key::Escape,
        "Tab" => egui::Key::Tab, "Backspace" => egui::Key::Backspace,
        "Delete" => egui::Key::Delete, "Insert" => egui::Key::Insert,
        "Home" => egui::Key::Home, "End" => egui::Key::End,
        "PageUp" => egui::Key::PageUp, "PageDown" => egui::Key::PageDown,
        "Space" => egui::Key::Space,
        "ArrowUp" => egui::Key::ArrowUp, "Up" => egui::Key::ArrowUp,
        "ArrowDown" => egui::Key::ArrowDown, "Down" => egui::Key::ArrowDown,
        "ArrowLeft" => egui::Key::ArrowLeft, "Left" => egui::Key::ArrowLeft,
        "ArrowRight" => egui::Key::ArrowRight, "Right" => egui::Key::ArrowRight,
        // Функциональные клавиши
        "F1" => egui::Key::F1, "F2" => egui::Key::F2, "F3" => egui::Key::F3,
        "F4" => egui::Key::F4, "F5" => egui::Key::F5, "F6" => egui::Key::F6,
        "F7" => egui::Key::F7, "F8" => egui::Key::F8, "F9" => egui::Key::F9,
        "F10" => egui::Key::F10, "F11" => egui::Key::F11, "F12" => egui::Key::F12,
        // Символы
        "Minus" => egui::Key::Minus, "Plus" => egui::Key::Plus,
        "Equals" => egui::Key::Equals, "Comma" => egui::Key::Comma,
        "Period" => egui::Key::Period, "Slash" => egui::Key::Slash,
        "Semicolon" => egui::Key::Semicolon, "Quote" => egui::Key::Quote,
        "Backslash" => egui::Key::Backslash,
        _ => return None,
    };
    Some(key)
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

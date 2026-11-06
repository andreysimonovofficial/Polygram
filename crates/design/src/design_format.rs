//! Сериализуемый формат дизайна Polygram.
//! Описывает полный визуальный стиль приложения: тему, стили по типам объектов,
//! контейнеры, линии, шрифты. Сохраняется в JSON и передаётся Агенту для точной отрисовки.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Корневой документ дизайна.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DesignDocument {
    pub version: u32,
    pub name: String,
    pub theme: ThemeDesign,
    pub object_styles: HashMap<String, ObjectStyle>,
    pub edge_style: EdgeStyle,
    pub container_style: ContainerStyle,
    pub typography: Typography,
}

impl Default for DesignDocument {
    fn default() -> Self {
        Self {
            version: 1,
            name: "Default Design".to_owned(),
            theme: ThemeDesign::default(),
            object_styles: HashMap::from([
                ("process".to_owned(), ObjectStyle::process_default()),
                ("department".to_owned(), ObjectStyle::department_default()),
                ("portal".to_owned(), ObjectStyle::portal_default()),
            ]),
            edge_style: EdgeStyle::default(),
            container_style: ContainerStyle::default(),
            typography: Typography::default(),
        }
    }
}

/// Цветовая тема холста.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThemeDesign {
    pub bg: Rgba,
    pub grid: Rgba,
    pub card_bg: Rgba,
    pub card_border: Rgba,
    pub card_header: Rgba,
    pub line: Rgba,
    pub line_hover: Rgba,
    pub status_ok: Rgba,
    pub status_warning: Rgba,
    pub status_deficit: Rgba,
    pub status_surplus: Rgba,
    pub status_unconnected: Rgba,
    pub status_invalid: Rgba,
    pub text: Rgba,
    pub text_muted: Rgba,
}

impl Default for ThemeDesign {
    fn default() -> Self {
        Self {
            bg: Rgba::from_rgb(18, 18, 22),
            grid: Rgba::from_rgb(35, 35, 42),
            card_bg: Rgba::from_rgb(28, 28, 34),
            card_border: Rgba::from_rgb(60, 60, 72),
            card_header: Rgba::from_rgb(45, 45, 54),
            line: Rgba::from_rgb(120, 120, 135),
            line_hover: Rgba::from_rgb(160, 160, 180),
            status_ok: Rgba::from_rgb(76, 175, 80),
            status_warning: Rgba::from_rgb(255, 193, 7),
            status_deficit: Rgba::from_rgb(244, 67, 54),
            status_surplus: Rgba::from_rgb(3, 169, 244),
            status_unconnected: Rgba::from_rgb(100, 100, 110),
            status_invalid: Rgba::from_rgb(158, 158, 158),
            text: Rgba::from_rgb(230, 230, 235),
            text_muted: Rgba::from_rgb(150, 150, 160),
        }
    }
}

/// RGBA цвет.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    #[serde(default = "default_alpha")]
    pub a: u8,
}

fn default_alpha() -> u8 {
    255
}

impl Rgba {
    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
    pub fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

/// Стиль отдельного объекта (карточки).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObjectStyle {
    pub kind: String,
    pub bg: Rgba,
    pub border: Rgba,
    pub border_width: f32,
    pub corner_radius: f32,
    pub header_bg: Rgba,
    pub header_height_cells: f32,
    pub title_color: Rgba,
    pub title_font_size: f32,
    pub id_color: Rgba,
    pub id_font_size: f32,
    pub container_bg: Rgba,
    pub container_border: Rgba,
    pub entry_height_cells: f32,
    pub port_radius: f32,
    pub port_color: Rgba,
    pub shadow: Option<Shadow>,
    pub glow: Option<Glow>,
}

impl ObjectStyle {
    pub fn process_default() -> Self {
        Self {
            kind: "process".to_owned(),
            bg: Rgba::from_rgb(28, 28, 34),
            border: Rgba::from_rgb(60, 60, 72),
            border_width: 1.0,
            corner_radius: 8.0,
            header_bg: Rgba::from_rgb(45, 45, 54),
            header_height_cells: 1.0,
            title_color: Rgba::from_rgb(230, 230, 235),
            title_font_size: 14.0,
            id_color: Rgba::from_rgb(150, 150, 160),
            id_font_size: 12.0,
            container_bg: Rgba::from_rgb(35, 35, 42),
            container_border: Rgba::from_rgb(50, 50, 60),
            entry_height_cells: 1.0,
            port_radius: 4.0,
            port_color: Rgba::from_rgb(100, 100, 110),
            shadow: None,
            glow: None,
        }
    }

    pub fn department_default() -> Self {
        Self {
            kind: "department".to_owned(),
            bg: Rgba::from_rgb(24, 24, 30),
            border: Rgba::from_rgb(70, 70, 85),
            border_width: 2.0,
            corner_radius: 6.0,
            header_bg: Rgba::from_rgb(40, 40, 48),
            header_height_cells: 1.2,
            title_color: Rgba::from_rgb(240, 240, 245),
            title_font_size: 16.0,
            id_color: Rgba::from_rgb(160, 160, 170),
            id_font_size: 12.0,
            container_bg: Rgba::from_rgb(30, 30, 36),
            container_border: Rgba::from_rgb(55, 55, 65),
            entry_height_cells: 1.0,
            port_radius: 4.0,
            port_color: Rgba::from_rgb(110, 110, 120),
            shadow: Some(Shadow {
                offset: [4.0, 4.0],
                blur: 8.0,
                color: Rgba::from_rgba(0, 0, 0, 80),
            }),
            glow: None,
        }
    }

    pub fn portal_default() -> Self {
        Self {
            kind: "portal".to_owned(),
            bg: Rgba::from_rgb(40, 40, 50),
            border: Rgba::from_rgb(100, 100, 120),
            border_width: 1.5,
            corner_radius: 12.0,
            header_bg: Rgba::from_rgb(50, 50, 60),
            header_height_cells: 1.0,
            title_color: Rgba::from_rgb(200, 200, 210),
            title_font_size: 13.0,
            id_color: Rgba::from_rgb(140, 140, 150),
            id_font_size: 11.0,
            container_bg: Rgba::from_rgb(45, 45, 55),
            container_border: Rgba::from_rgb(70, 70, 80),
            entry_height_cells: 1.0,
            port_radius: 5.0,
            port_color: Rgba::from_rgb(120, 120, 130),
            shadow: None,
            glow: Some(Glow {
                color: Rgba::from_rgba(100, 100, 200, 60),
                size: 6.0,
            }),
        }
    }
}

/// Стиль линий (рёбер).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EdgeStyle {
    pub color: Rgba,
    pub width: f32,
    pub hover_width: f32,
    pub hover_color: Rgba,
    pub corner_radius: f32,
    pub dash_pattern: Option<Vec<f32>>,
}

impl Default for EdgeStyle {
    fn default() -> Self {
        Self {
            color: Rgba::from_rgb(120, 120, 135),
            width: 1.5,
            hover_width: 2.5,
            hover_color: Rgba::from_rgb(160, 160, 180),
            corner_radius: 4.0,
            dash_pattern: None,
        }
    }
}

/// Стиль контейнера показателей.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainerStyle {
    pub bg: Rgba,
    pub border: Rgba,
    pub border_width: f32,
    pub corner_radius: f32,
    pub header_height_cells: f32,
    pub header_bg: Rgba,
    pub entry_spacing: f32,
}

impl Default for ContainerStyle {
    fn default() -> Self {
        Self {
            bg: Rgba::from_rgb(35, 35, 42),
            border: Rgba::from_rgb(50, 50, 60),
            border_width: 1.0,
            corner_radius: 4.0,
            header_height_cells: 0.5,
            header_bg: Rgba::from_rgb(45, 45, 54),
            entry_spacing: 2.0,
        }
    }
}

/// Типографика.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Typography {
    pub font_size: f32,
    pub title_font_size: f32,
    pub id_font_size: f32,
    pub label_font_size: f32,
    pub line_height: f32,
}

impl Default for Typography {
    fn default() -> Self {
        Self {
            font_size: 13.0,
            title_font_size: 14.0,
            id_font_size: 12.0,
            label_font_size: 11.0,
            line_height: 1.4,
        }
    }
}

/// Тень.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Shadow {
    pub offset: [f32; 2],
    pub blur: f32,
    pub color: Rgba,
}

/// Свечение.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Glow {
    pub color: Rgba,
    pub size: f32,
}

fn default_true() -> bool {
    true
}

/// Как рамка-контейнер распространяет наведение/нажатие на детей.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChildReactMode {
    /// Все дети с reacts_to_parent = true горят вместе с родителем (по умолчанию).
    All,
    /// Только дети из child_react_allow + reacts_to_parent.
    List,
    /// Дети не горят с родителем (только прямое попадание).
    None,
}

impl Default for ChildReactMode {
    fn default() -> Self {
        ChildReactMode::All
    }
}

/// Инструмент рисования свободной фигуры (аналог Figma).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DrawTool {
    Select,
    Rect,
    Line,
    Ellipse,
    Text,
}

/// Визуальное состояние элемента.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShapeState {
    Normal,
    Hover,
    Pressed,
}

/// Переопределение стиля для состояния наведения или нажатия.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct StateStyle {
    pub fill: Option<Rgba>,
    pub stroke: Option<Rgba>,
    pub stroke_width: Option<f32>,
}

/// Стили для состояний наведения и нажатия (нормальное состояние — основные поля фигуры).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ShapeStates {
    pub hover: Option<StateStyle>,
    pub pressed: Option<StateStyle>,
}

/// Свободная фигура с абсолютной позицией (в пикселях) — отрисовывается агентом в точности.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FreeShape {
    pub id: String,
    pub tool: DrawTool,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub fill: Option<Rgba>,
    pub stroke: Rgba,
    pub stroke_width: f32,
    pub corner_radius: f32,
    pub corner_tl: f32,
    pub corner_tr: f32,
    pub corner_bl: f32,
    pub corner_br: f32,
    pub text: String,
    pub font_size: f32,
    pub text_color: Rgba,
    pub text_align: TextAlign,
    /// Переопределения стилей для состояний наведения и нажатия (режим прототипа).
    #[serde(default)]
    pub states: ShapeStates,
    /// Идентификатор группы. Фигуры с одинаковым group_id реагируют на наведение/нажатие вместе.
    #[serde(default)]
    pub group_id: Option<String>,
    /// Логический родитель (рамка/контейнер). Если Some — фигура дочерняя.
    #[serde(default)]
    pub parent_id: Option<String>,
    /// Является ли фигура рамкой-контейнером (распространяет состояние на детей).
    #[serde(default)]
    pub is_container: bool,
    /// Реагировать ли этому ребёнку на состояние родителя (по умолчанию true).
    #[serde(default = "default_true")]
    pub reacts_to_parent: bool,
    /// Как рамка распространяет состояние на детей.
    #[serde(default)]
    pub child_react_mode: ChildReactMode,
    /// Список id детей, которые реагируют, когда child_react_mode == List.
    #[serde(default)]
    pub child_react_allow: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

impl Default for FreeShape {
    fn default() -> Self {
        Self {
            id: String::new(),
            tool: DrawTool::Rect,
            x: 0.0,
            y: 0.0,
            w: 120.0,
            h: 80.0,
            fill: Some(Rgba { r: 0, g: 0, b: 0, a: 255 }),
            stroke: Rgba { r: 255, g: 255, b: 255, a: 255 },
            stroke_width: 1.0,
            corner_radius: 0.0,
            corner_tl: 0.0,
            corner_tr: 0.0,
            corner_bl: 0.0,
            corner_br: 0.0,
            text: String::new(),
            font_size: 14.0,
            text_color: Rgba::from_rgb(230, 230, 235),
            text_align: TextAlign::Left,
            states: ShapeStates::default(),
            group_id: None,
            parent_id: None,
            is_container: false,
            reacts_to_parent: true,
            child_react_mode: ChildReactMode::All,
            child_react_allow: Vec::new(),
        }
    }
}

/// Документ свободного холста (Figma-подобный).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FreeCanvasDoc {
    pub version: u32,
    pub name: String,
    pub bg: Rgba,
    pub grid: Rgba,
    pub show_grid: bool,
    pub shapes: Vec<FreeShape>,
    /// Режим прототипа: симуляция наведения/нажатия с визуальными изменениями.
    #[serde(default)]
    pub prototype_mode: bool,
}

impl Default for FreeCanvasDoc {
    fn default() -> Self {
        Self {
            version: 1,
            name: "Free Canvas".to_owned(),
            bg: Rgba::from_rgb(18, 18, 22),
            grid: Rgba::from_rgb(35, 35, 42),
            show_grid: true,
            shapes: Vec::new(),
            prototype_mode: false,
        }
    }
}

impl FreeCanvasDoc {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn load_from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl DesignDocument {
    pub fn load_from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

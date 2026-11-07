//! Рендер объекта "Процесс" по шаблону design.json (полная копия макета со stub-кнопками).
//!
//! Стратегия:
//! - Шаблон design.json — это FreeCanvasDoc с фигурами в абсолютных пикселях
//!   макета (базовый размер 372×380). Каждая figura отображается в экранные
//!   координаты внутри rect процесса с линейным масштабированием.
//! - Обводка рисуется константной толщиной в экранных пикселях (не меняется от зума).
//! - Текст рисуется через painter.text() с galley (замер ширины), шрифт Inter.
//! - Интерактивные зоны ("≡", "+ Добавить показатель") зарегистрированы как
//!   hit-тест заглушки — функциональность будет добавлена позже.

use crate::theme::Theme;
use egui::{Color32, FontId, Painter, Pos2, Rect, Stroke, Vec2};
use polygram_core::{DynamicState, ObjectGeom};
use serde::Deserialize;
use std::sync::OnceLock;

/// Базовый размер макета design.json (в пикселях макета).
const DESIGN_W: f32 = 372.0;
const DESIGN_H: f32 = 380.0;

/// Точка в формате макета.
#[derive(Clone, Copy, Debug, Deserialize)]
struct LayoutPoint {
    pub x: f32,
    pub y: f32,
}

/// Цвет RGBA в формате макета.
#[derive(Clone, Copy, Debug, Deserialize)]
struct LayoutRgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    #[serde(default = "default_alpha")]
    pub a: u8,
}

fn default_alpha() -> u8 {
    255
}

impl LayoutRgba {
    fn to_color32(self) -> Color32 {
        Color32::from_rgba_unmultiplied(self.r, self.g, self.b, self.a)
    }
}

/// Позиция обводки.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
enum LayoutStrokePosition {
    #[default]
    Center,
    Inside,
    Outside,
}

/// Толщина шрифта.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
enum LayoutFontWeight {
    Thin,
    ExtraLight,
    Light,
    #[default]
    Regular,
    Medium,
    SemiBold,
    Bold,
    ExtraBold,
    Black,
}

/// Семейство шрифта.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
enum LayoutFontFamily {
    #[default]
    Inter,
    SegoeUI,
}

/// Выравнивание по горизонтали.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
enum LayoutTextAlign {
    Left,
    Center,
    Right,
}

/// Выравнивание по вертикали.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
enum LayoutTextAlignV {
    Top,
    Middle,
    Bottom,
}

/// Инструмент фигуры.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
enum LayoutDrawTool {
    Select,
    Rect,
    Line,
    Ellipse,
    Text,
}

/// Состояние для наведения/нажатия.
#[derive(Clone, Debug, Default, Deserialize)]
struct LayoutStateStyle {
    pub fill: Option<LayoutRgba>,
    pub stroke: Option<LayoutRgba>,
    pub stroke_width: Option<f32>,
}

/// Состояния фигуры.
#[derive(Clone, Debug, Default, Deserialize)]
struct LayoutShapeStates {
    pub hover: Option<LayoutStateStyle>,
    pub pressed: Option<LayoutStateStyle>,
}

/// Одна фигура из design.json.
#[derive(Clone, Debug, Deserialize)]
struct LayoutShape {
    pub id: String,
    pub tool: LayoutDrawTool,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub fill: Option<LayoutRgba>,
    pub stroke: LayoutRgba,
    pub stroke_width: f32,
    #[serde(default)]
    pub stroke_position: LayoutStrokePosition,
    pub corner_radius: f32,
    pub corner_tl: f32,
    pub corner_tr: f32,
    pub corner_bl: f32,
    pub corner_br: f32,
    pub text: String,
    pub font_size: f32,
    pub text_color: LayoutRgba,
    pub text_align: LayoutTextAlign,
    pub text_align_v: LayoutTextAlignV,
    #[serde(default)]
    pub font_weight: LayoutFontWeight,
    #[serde(default)]
    pub font_family: LayoutFontFamily,
    #[serde(default)]
    pub states: LayoutShapeStates,
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub is_container: bool,
    #[serde(default = "default_true")]
    pub reacts_to_parent: bool,
    #[serde(default)]
    pub child_react_mode: String,
    #[serde(default)]
    pub child_react_allow: Vec<String>,
}

fn default_true() -> bool {
    true
}

/// Весь документ дизайна процесса.
#[derive(Clone, Debug, Deserialize)]
struct LayoutDoc {
    pub version: u32,
    pub name: String,
    pub bg: LayoutRgba,
    pub grid: LayoutRgba,
    pub show_grid: bool,
    pub shapes: Vec<LayoutShape>,
    #[serde(default)]
    pub prototype_mode: bool,
}

/// Загрузить шаблон процесса из встроенного design.json.
fn load_process_layout() -> LayoutDoc {
    let json = include_str!("process_design.json");
    serde_json::from_str(json).expect("Failed to parse process_design.json")
}

/// Hit-тест зоны интерактивных элементов (stub).
/// Каждая зона: прямоугольник в экранных координатах + идентификатор действия.
#[derive(Clone, Debug)]
pub struct ProcessHitZone {
    pub rect: Rect,
    pub action: ProcessAction,
}

/// Действия stub-кнопок процесса (реализация позже).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessAction {
    /// Кнопка меню "≡" в контейнере Запрос.
    RequestMenu,
    /// Кнопка меню "≡" в контейнере Выход.
    OutputMenu,
    /// Ссылка "+ Добавить показатель" в контейнере Запрос.
    AddRequestMetric,
    /// Ссылка "+ Добавить показатель" в контейнере Выход.
    AddOutputMetric,
}

/// Собранный шаблон процесса: фигуры + hit-тест зоны.
#[derive(Clone, Debug)]
pub struct ProcessTemplate {
    pub doc: LayoutDoc,
    pub hit_zones: Vec<ProcessHitZone>,
}

impl ProcessTemplate {
    pub fn load() -> Self {
        let doc = load_process_layout();
        let mut hit_zones = Vec::new();

        for shape in &doc.shapes {
            match shape.id.as_str() {
                "ui-7" | "shape-2" => {
                    hit_zones.push(ProcessHitZone {
                        rect: Rect::from_min_size(
                            Pos2::new(shape.x, shape.y),
                            Vec2::new(shape.w.max(1.0), shape.h.max(1.0)),
                        ),
                        action: ProcessAction::RequestMenu,
                    });
                }
                "ui-13" | "shape-3" => {
                    hit_zones.push(ProcessHitZone {
                        rect: Rect::from_min_size(
                            Pos2::new(shape.x, shape.y),
                            Vec2::new(shape.w.max(1.0), shape.h.max(1.0)),
                        ),
                        action: ProcessAction::OutputMenu,
                    });
                }
                "ui-9" => {
                    hit_zones.push(ProcessHitZone {
                        rect: Rect::from_min_size(
                            Pos2::new(shape.x, shape.y),
                            Vec2::new(shape.w.max(1.0), shape.h.max(1.0)),
                        ),
                        action: ProcessAction::AddRequestMetric,
                    });
                }
                "ui-15" => {
                    hit_zones.push(ProcessHitZone {
                        rect: Rect::from_min_size(
                            Pos2::new(shape.x, shape.y),
                            Vec2::new(shape.w.max(1.0), shape.h.max(1.0)),
                        ),
                        action: ProcessAction::AddOutputMetric,
                    });
                }
                _ => {}
            }
        }

        Self { doc, hit_zones }
    }
}

/// Текущий загруженный шаблон (одна экземпляр на все процессы).
static PROCESS_TEMPLATE: OnceLock<ProcessTemplate> = OnceLock::new();

fn get_template() -> &'static ProcessTemplate {
    PROCESS_TEMPLATE.get_or_init(ProcessTemplate::load)
}

/// Разрешить цвет fill фигуры с учётом состояния (пока только Normal — заглушка).
fn resolve_fill(shape: &LayoutShape) -> Color32 {
    shape
        .fill
        .map(|c| c.to_color32())
        .unwrap_or(Color32::TRANSPARENT)
}

/// Разрешить цвет обводки фигуры.
fn resolve_stroke_color(shape: &LayoutShape) -> Color32 {
    shape.stroke.to_color32()
}

/// Разрешить толщину обводки (без масштабирования на зум — константная в экранных пикселях).
fn resolve_stroke_width(shape: &LayoutShape) -> f32 {
    shape.stroke_width.max(0.0)
}

/// Получить FontId для фигуры (шрифт Inter, фиксированный размер в экранных пикселях,
/// не меняющийся при зуме).
fn layout_font_id(shape: &LayoutShape) -> FontId {
    let size = shape.font_size.max(6.0).max(8.0);
    let weight = match shape.font_weight {
        LayoutFontWeight::Thin => egui::FontFamily::Name("inter-thin".into()),
        LayoutFontWeight::ExtraLight => egui::FontFamily::Name("inter-extralight".into()),
        LayoutFontWeight::Light => egui::FontFamily::Name("inter-light".into()),
        LayoutFontWeight::Regular => egui::FontFamily::Name("inter-regular".into()),
        LayoutFontWeight::Medium => egui::FontFamily::Name("inter-medium".into()),
        LayoutFontWeight::SemiBold => egui::FontFamily::Name("inter-semibold".into()),
        LayoutFontWeight::Bold => egui::FontFamily::Name("inter-bold".into()),
        LayoutFontWeight::ExtraBold => egui::FontFamily::Name("inter-extrabold".into()),
        LayoutFontWeight::Black => egui::FontFamily::Name("inter-black".into()),
    };
    FontId::new(size, weight)
}

/// Вычислить скругление фигуры.
fn layout_rounding(shape: &LayoutShape) -> egui::Rounding {
    if shape.corner_tl == shape.corner_tr
        && shape.corner_tr == shape.corner_bl
        && shape.corner_bl == shape.corner_br
    {
        egui::Rounding::same(shape.corner_radius)
    } else {
        egui::Rounding {
            nw: shape.corner_tl,
            ne: shape.corner_tr,
            sw: shape.corner_bl,
            se: shape.corner_br,
        }
    }
}

/// Скорректировать rect под позицию обводки (как в Design Tool).
fn stroke_rect_for_position(rect: Rect, stroke_width: f32, position: LayoutStrokePosition) -> Rect {
    let max_shrink = (rect.width().min(rect.height()) / 2.0).max(0.0);
    let shrink = match position {
        LayoutStrokePosition::Outside => 0.0,
        LayoutStrokePosition::Center => (stroke_width / 2.0).min(max_shrink),
        LayoutStrokePosition::Inside => stroke_width.min(max_shrink),
    };
    rect.shrink(shrink)
}

/// Рисовать одну фигуру из шаблона design.json.
fn paint_layout_shape(
    painter: &Painter,
    shape: &LayoutShape,
    screen_rect: Rect,
) {
    // Пропускать фигуры с некорректным размером, чтобы избежать паники в egui.
    if screen_rect.width() <= 0.0 || screen_rect.height() <= 0.0 {
        return;
    }
    let fill = resolve_fill(shape);
    let stroke_color = resolve_stroke_color(shape);
    let stroke_width = resolve_stroke_width(shape);
    let stroke = Stroke::new(stroke_width, stroke_color);

    match shape.tool {
        LayoutDrawTool::Select => {}
        LayoutDrawTool::Rect | LayoutDrawTool::Text => {
            painter.rect_filled(screen_rect, layout_rounding(shape), fill);
            if stroke_width > 0.0 {
                let stroke_rect = stroke_rect_for_position(screen_rect, stroke_width, shape.stroke_position);
                painter.rect_stroke(stroke_rect, layout_rounding(shape), stroke);
            }
        }
        LayoutDrawTool::Ellipse => {
            let center = screen_rect.center();
            let fill_radii = Vec2::new(screen_rect.width() / 2.0, screen_rect.height() / 2.0);
            painter.add(egui::Shape::ellipse_filled(center, fill_radii, fill));
            if stroke_width > 0.0 {
                let radii = fill_radii - Vec2::splat(stroke_width.min(fill_radii.x.min(fill_radii.y)));
                painter.add(egui::Shape::ellipse_stroke(center, radii, stroke));
            }
        }
        LayoutDrawTool::Line => {
            let p1 = Pos2::new(screen_rect.min.x, screen_rect.min.y);
            let p2 = Pos2::new(screen_rect.max.x, screen_rect.max.y);
            painter.line_segment([p1, p2], stroke);
        }
    }

    // Текст
    if !shape.text.is_empty() {
        let text_color = shape.text_color.to_color32();
        let font_id = layout_font_id(shape);
        let galley = painter.layout_no_wrap(shape.text.clone(), font_id, text_color);
        let text_w = galley.size().x;
        let text_h = galley.size().y;
        let x = match shape.text_align {
            LayoutTextAlign::Left => screen_rect.min.x,
            LayoutTextAlign::Center => screen_rect.center().x - text_w / 2.0,
            LayoutTextAlign::Right => screen_rect.max.x - text_w,
        };
        let y = match shape.text_align_v {
            LayoutTextAlignV::Top => screen_rect.min.y,
            LayoutTextAlignV::Middle => screen_rect.center().y - text_h / 2.0,
            LayoutTextAlignV::Bottom => screen_rect.max.y - text_h,
        };
        painter.galley(Pos2::new(x, y), galley, text_color);
    }
}

/// Высокопроизводительный рендер объекта "Процесс" по макету design.json.
pub fn paint_process(
    painter: &Painter,
    _object: &ObjectGeom,
    _dynamic: &DynamicState,
    _theme: &Theme,
    rect_px: Rect,
    _cell_px: f32,
    _is_hovered: bool,
) {
    let template = get_template();

    // Масштаб макета → экран (линейный, по обеим осям независимо).
    let scale_x = rect_px.width() / DESIGN_W;
    let scale_y = rect_px.height() / DESIGN_H;
    let offset = rect_px.min;

    // Отрисовать все фигуры шаблона.
    for shape in &template.doc.shapes {
        let screen_rect = Rect::from_min_size(
            Pos2::new(
                offset.x + shape.x * scale_x,
                offset.y + shape.y * scale_y,
            ),
            Vec2::new(shape.w.max(1.0) * scale_x, shape.h.max(1.0) * scale_y),
        );
        paint_layout_shape(painter, shape, screen_rect);
    }

    // TODO: hit-тест зоны доступны в template.hit_zones — будут использованы
    // после того, как будет определено, за что каждая кнопка отвечает.
    let _ = &template.hit_zones;
}

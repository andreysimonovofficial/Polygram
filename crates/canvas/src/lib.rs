//! Холст Polygram: рендер объектов и ортогональных линий со снимков ядра.
//! Визуальный дизайн: простые примитивы, подсветка только проблемных элементов,
//! линии от элемента к элементу, кнопки списков показателей.

mod camera;
mod hit_test;
mod paint_department;
mod paint_edge;
mod paint_object;
mod paint_process;
mod theme;

pub use camera::Camera;
pub use hit_test::{header_at, object_at};
pub use paint_department::paint_department_frame;
pub use paint_edge::paint_edge;
pub use paint_object::paint_object;
pub use paint_process::paint_process;
pub use paint_process::paint_drag_preview;
pub use theme::Theme;

use egui::{Color32, Pos2, Rect, Vec2};
use polygram_core::layout::Rect as CoreRect;
use polygram_core::types::{Direction, NodeKind};
use polygram_core::{DynamicState, Geometry, StatusCode};
use std::collections::HashMap;

pub use paint_process::MetricBinding;

/// Холст.
pub struct Canvas {
    pub theme: Theme,
    pub camera: Camera,
    pub cell_size: f32,
    /// Горячие зоны добавления показателя (id объекта, направление, экранный rect).
    pub add_metric_hotspots: Vec<(String, Direction, Rect)>,
    /// Горячие зоны кнопки меню контейнера (id объекта, направление, экранный rect кнопки ≡).
    pub container_menu_hotspots: Vec<(String, Direction, Rect)>,
    /// Горячие зоны строк показателей для перетаскивания (id объекта, направление, имя показателя, экранный rect строки).
    pub drag_metric_hotspots: Vec<(String, Direction, String, Rect)>,
    /// Горячие зоны иконок ошибок API-привязки (экранный rect, текст ошибки для копирования).
    pub api_error_hotspots: Vec<(Rect, String)>,
}

impl Default for Canvas {
    fn default() -> Self {
        Self::new()
    }
}

impl Canvas {
    pub fn new() -> Self {
        Self {
            theme: Theme::default(),
            camera: Camera::new(),
            cell_size: 16.0,
            add_metric_hotspots: Vec::new(),
            container_menu_hotspots: Vec::new(),
            drag_metric_hotspots: Vec::new(),
            api_error_hotspots: Vec::new(),
        }
    }

    pub fn cell_px(&self) -> f32 {
        self.cell_size * self.camera.zoom
    }

    /// Главная функция рендера.
    pub fn paint(
        &mut self,
        ui: &mut egui::Ui,
        geometry: &Geometry,
        dynamic: &DynamicState,
        hovered_object: Option<&str>,
        edit_target: Option<&str>,
        selected_objects: &[String],
        show_grid: bool,
        bindings: &HashMap<(String, String), MetricBinding>,
    ) {
        let painter = ui.painter();
        let clip_rect = ui.clip_rect();
        let cell_px = self.cell_px();
        self.add_metric_hotspots.clear();
        self.container_menu_hotspots.clear();
        self.drag_metric_hotspots.clear();
        self.api_error_hotspots.clear();
        let mut error_hotspots: Vec<(Rect, String)> = Vec::new();

        // Фон холста.
        painter.rect_filled(clip_rect, 0.0, self.theme.bg);

        // Сетка — нижний слой, после фона.
        if show_grid {
            let cell_size = self.cell_size;
            let zoom = self.camera.zoom;
            let offset = self.camera.offset;
            let grid_color = Color32::from_rgb(35, 35, 42);

            let min_cx = ((clip_rect.min.x / zoom - offset.x) / cell_size).floor() as i32;
            let max_cx = ((clip_rect.max.x / zoom - offset.x) / cell_size).ceil() as i32;
            let min_cy = ((clip_rect.min.y / zoom - offset.y) / cell_size).floor() as i32;
            let max_cy = ((clip_rect.max.y / zoom - offset.y) / cell_size).ceil() as i32;

            for c in min_cx..=max_cx {
                let sx = (c as f32 * cell_size + offset.x) * zoom;
                painter.line_segment(
                    [Pos2::new(sx, clip_rect.min.y), Pos2::new(sx, clip_rect.max.y)],
                    egui::Stroke::new(1.0_f32, grid_color),
                );
            }
            for r in min_cy..=max_cy {
                let sy = (r as f32 * cell_size + offset.y) * zoom;
                painter.line_segment(
                    [Pos2::new(clip_rect.min.x, sy), Pos2::new(clip_rect.max.x, sy)],
                    egui::Stroke::new(1.0_f32, grid_color),
                );
            }
        }

        // Отделы — рамки-контейнеры (под дочерними карточками)
        for object in &geometry.objects {
            if object.kind != NodeKind::Department {
                continue;
            }
            let r = CoreRect::new(object.rect.x, object.rect.y, object.rect.w, object.rect.h);
            let rect_px = Rect::from_min_size(
                self.camera
                    .world_to_screen(Pos2::new(r.x as f32, r.y as f32), self.cell_size),
                Vec2::new(r.w as f32 * cell_px, r.h as f32 * cell_px),
            );
            if !rect_px.intersects(clip_rect) {
                continue;
            }
            paint_department_frame(
                painter,
                object,
                rect_px,
                cell_px,
                self.camera.zoom,
                &self.theme,
            );
        }

        // Потом карточки объектов (включая дочерние внутри отделов)
        for object in &geometry.objects {
            if object.kind == NodeKind::Department {
                continue;
            }
            let is_hovered = hovered_object == Some(&object.id);
            let r = CoreRect::new(object.rect.x, object.rect.y, object.rect.w, object.rect.h);
            let rect_px = Rect::from_min_size(
                self.camera
                    .world_to_screen(Pos2::new(r.x as f32, r.y as f32), self.cell_size),
                Vec2::new(r.w as f32 * cell_px, r.h as f32 * cell_px),
            );
            if !rect_px.intersects(clip_rect) {
                continue;
            }
            if object.kind == NodeKind::Process {
                let (add_hotspots, menu_hotspots, drag_hotspots) = paint_process(
                    painter,
                    object,
                    dynamic,
                    &self.theme,
                    rect_px,
                    cell_px,
                    self.camera.zoom,
                    is_hovered,
                    edit_target,
                    bindings,
                    &mut error_hotspots,
                );
                self.add_metric_hotspots.extend(add_hotspots);
                self.container_menu_hotspots.extend(menu_hotspots);
                self.drag_metric_hotspots.extend(drag_hotspots);
            } else {
                paint_object(
                    painter,
                    object,
                    dynamic,
                    &self.theme,
                    rect_px,
                    cell_px,
                    self.camera.zoom,
                    is_hovered,
                );
            }
        }

        self.api_error_hotspots = error_hotspots;

        // Линии связи рисуются поверх объектов, чтобы объекты не загораживали их.
        for edge in &geometry.edges {
            paint_edge(
                painter,
                edge,
                geometry,
                dynamic,
                &self.theme,
                cell_px,
                self.camera.offset,
                self.camera.zoom,
            );
        }

        // Рамки выделения вокруг выделенных объектов (рисуются поверх всего).
        if !selected_objects.is_empty() {
            for object in &geometry.objects {
                if !selected_objects.iter().any(|id| id == &object.id) {
                    continue;
                }
                let r = CoreRect::new(object.rect.x, object.rect.y, object.rect.w, object.rect.h);
                let rect_px = Rect::from_min_size(
                    self.camera
                        .world_to_screen(Pos2::new(r.x as f32, r.y as f32), self.cell_size),
                    Vec2::new(r.w as f32 * cell_px, r.h as f32 * cell_px),
                );
                if !rect_px.intersects(clip_rect) {
                    continue;
                }
                let sr = rect_px.expand(4.0);
                painter.rect_stroke(
                    sr,
                    2.0,
                    egui::Stroke::new(2.0_f32 * self.camera.zoom, egui::Color32::from_rgb(100, 180, 255)),
                );
            }
        }
    }

    pub fn object_at<'a>(&self, geometry: &'a Geometry, screen_pos: Pos2) -> Option<&'a str> {
        object_at(
            geometry,
            screen_pos,
            self.camera.offset,
            self.camera.zoom,
            self.cell_size,
        )
    }

    pub fn header_at<'a>(&self, geometry: &'a Geometry, screen_pos: Pos2) -> Option<&'a str> {
        header_at(
            geometry,
            screen_pos,
            self.camera.offset,
            self.camera.zoom,
            self.cell_size,
        )
    }
}

/// Цвет точки статуса и линии связи с учётом направления (Запрос / Выход).
/// Запрос: Удовлетворён(Ok) = Зелёный, Ожидание(Deficit) = Красный.
/// Выход: Обеспечен(Ok) = Зелёный, Избыток(Surplus) = Жёлтый, Ожидание(Deficit) = Красный,
/// В процессе(Warning) = Синий.
pub fn status_dot_color(status: StatusCode, dir: Direction, theme: &Theme) -> Color32 {
    match dir {
        Direction::Request => match status {
            StatusCode::Ok => theme.status_ok,
            StatusCode::Deficit => theme.status_deficit,
            _ => theme.text_muted,
        },
        Direction::Output => match status {
            StatusCode::Ok => theme.status_ok,
            StatusCode::Surplus => theme.status_warning,
            StatusCode::Deficit => theme.status_deficit,
            StatusCode::Warning => theme.status_surplus,
            _ => theme.text_muted,
        },
    }
}

/// Метка статуса по-русски с учётом направления (всегда верхний регистр).
pub fn status_label_ru(status: StatusCode, dir: Direction) -> &'static str {
    match dir {
        Direction::Request => match status {
            StatusCode::Ok => "УДОВЛЕТВОРЁН",
            StatusCode::Deficit => "ОЖИДАНИЕ",
            StatusCode::Unconnected => "НЕ ПОДКЛЮЧЕНО",
            _ => "НОРМА",
        },
        Direction::Output => match status {
            StatusCode::Ok => "ОБЕСПЕЧЕН",
            StatusCode::Surplus => "ИЗБЫТОК",
            StatusCode::Deficit => "В ПРОЦЕССЕ",
            StatusCode::Unconnected => "НЕ ПОДКЛЮЧЕНО",
            _ => "НОРМА",
        },
    }
}

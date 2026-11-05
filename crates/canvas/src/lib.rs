//! Холст Polygram: рендер объектов и ортогональных линий со снимков ядра.
//! Визуальный дизайн: простые примитивы, подсветка только проблемных элементов,
//! линии от элемента к элементу, кнопки списков показателей.

mod camera;
mod hit_test;
mod paint_department;
mod paint_edge;
mod paint_object;
mod theme;

pub use camera::Camera;
pub use hit_test::{header_at, object_at};
pub use paint_department::paint_department_frame;
pub use paint_edge::paint_edge;
pub use paint_object::{paint_container, paint_object};
pub use theme::Theme;

use egui::{Color32, Pos2, Rect, Shape, Stroke, Vec2};
use polygram_core::{DynamicState, Geometry, StatusCode};
use polygram_core::layout::Rect as CoreRect;
use polygram_core::types::{Direction, NodeKind};

/// Холст.
pub struct Canvas {
    pub theme: Theme,
    pub camera: Camera,
    pub cell_size: f32,
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
        }
    }

    pub fn cell_px(&self) -> f32 {
        self.cell_size * self.camera.zoom
    }

    fn paint_grid(&self, painter: &egui::Painter, clip_rect: Rect) {
        let cell = self.cell_px();
        let offset = self.camera.offset;
        let zoom = self.camera.zoom;

        let min_w = (clip_rect.min.x / zoom - offset.x) / cell;
        let max_w = (clip_rect.max.x / zoom - offset.x) / cell;
        let min_h = (clip_rect.min.y / zoom - offset.y) / cell;
        let max_h = (clip_rect.max.y / zoom - offset.y) / cell;

        let mut shapes = Vec::new();
        for x in (min_w.floor() as i32)..=(max_w.ceil() as i32) {
            let sx = (x as f32 * cell + offset.x) * zoom;
            shapes.push(Shape::line(
                vec![Pos2::new(sx, clip_rect.min.y), Pos2::new(sx, clip_rect.max.y)],
                Stroke::new(1.0_f32, self.theme.grid),
            ));
        }
        for y in (min_h.floor() as i32)..=(max_h.ceil() as i32) {
            let sy = (y as f32 * cell + offset.y) * zoom;
            shapes.push(Shape::line(
                vec![Pos2::new(clip_rect.min.x, sy), Pos2::new(clip_rect.max.x, sy)],
                Stroke::new(1.0_f32, self.theme.grid),
            ));
        }
        painter.extend(shapes);
    }

    /// Главная функция рендера.
    pub fn paint(
        &mut self,
        ui: &mut egui::Ui,
        geometry: &Geometry,
        dynamic: &DynamicState,
        hovered_object: Option<&str>,
    ) {
        let painter = ui.painter();
        let clip_rect = ui.clip_rect();
        let cell_px = self.cell_px();

        self.paint_grid(painter, clip_rect);

        // Сначала линии (под карточками и отделами)
        for edge in &geometry.edges {
            paint_edge(
                painter, edge, geometry, dynamic, &self.theme,
                cell_px, self.camera.offset, self.camera.zoom,
            );
        }

        // Отделы — рамки-контейнеры (под дочерними карточками)
        for object in &geometry.objects {
            if object.kind != NodeKind::Department {
                continue;
            }
            let r = CoreRect::new(object.rect.x, object.rect.y, object.rect.w, object.rect.h);
            let rect_px = Rect::from_min_size(
                self.camera.world_to_screen(Pos2::new(r.x as f32, r.y as f32), cell_px),
                Vec2::new(r.w as f32 * cell_px, r.h as f32 * cell_px),
            );
            if !rect_px.intersects(clip_rect) {
                continue;
            }
            paint_department_frame(painter, object, rect_px, cell_px, &self.theme);
        }

        // Потом карточки объектов (включая дочерние внутри отделов)
        for object in &geometry.objects {
            if object.kind == NodeKind::Department {
                continue;
            }
            let is_hovered = hovered_object == Some(&object.id);
            let r = CoreRect::new(object.rect.x, object.rect.y, object.rect.w, object.rect.h);
            let rect_px = Rect::from_min_size(
                self.camera.world_to_screen(Pos2::new(r.x as f32, r.y as f32), cell_px),
                Vec2::new(r.w as f32 * cell_px, r.h as f32 * cell_px),
            );
            if !rect_px.intersects(clip_rect) {
                continue;
            }
            paint_object(
                painter, object, dynamic, &self.theme, rect_px, cell_px, is_hovered,
            );
        }
    }

    pub fn object_at<'a>(&self, geometry: &'a Geometry, screen_pos: Pos2) -> Option<&'a str> {
        object_at(geometry, screen_pos, self.camera.offset, self.camera.zoom, self.cell_size)
    }

    pub fn header_at<'a>(&self, geometry: &'a Geometry, screen_pos: Pos2) -> Option<&'a str> {
        header_at(geometry, screen_pos, self.camera.offset, self.camera.zoom, self.cell_size)
    }
}

/// Цвет точки статуса: серый для OK, цветной только для проблем.
pub fn status_dot_color(status: StatusCode, theme: &Theme) -> Color32 {
    match status {
        StatusCode::Ok => theme.text_muted,
        StatusCode::Warning => theme.status_warning,
        StatusCode::Deficit => theme.status_deficit,
        StatusCode::Surplus => theme.status_surplus,
        StatusCode::Unconnected => theme.status_unconnected,
        StatusCode::Invalid => theme.status_invalid,
        StatusCode::Disabled => theme.text_muted,
        StatusCode::Unknown => theme.text_muted,
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

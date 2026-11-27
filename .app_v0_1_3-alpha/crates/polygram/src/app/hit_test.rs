//! Hit-testing: определение объекта/шапки/отдела под заданной экранной позицией.
//! Отвечает за:
//! - объект_under — отдел под позицией (внутри rect отдела);
//! - header_under — шапка отдела или процесса под позицией;
//! - container_header_under — шапка контейнера под позицией;
//! - process_under — процесс под позицией (динамическая CSS-геометрия).

use egui::Pos2;
use polygram_canvas::paint::constants::PROCESS_HEADER_H_PX;
use polygram_core::types::{ContainerDirection, Direction, NodeKind};
use crate::app::PolygramApp;

impl PolygramApp {
    /// Найти отдел под заданной экранной позицией (внутри rect отдела).
    pub(crate) fn object_under(&self, screen_pos: Pos2) -> Option<String> {
        let cell_px = self.canvas.cell_px();
        for object in &self.canvas.objects {
            if object.kind != NodeKind::Department {
                continue;
            }
            let r = &object.rect;
            let rect_px = egui::Rect::from_min_size(
                self.canvas.cell_to_screen(r.x, r.y),
                egui::Vec2::new(r.w as f32 * cell_px, r.h as f32 * cell_px),
            );
            if rect_px.contains(screen_pos) {
                return Some(object.id.clone());
            }
        }
        None
    }

    /// Найти отдел, чья шапка содержит заданную экранную позицию.
    pub(crate) fn header_under(&self, screen_pos: Pos2) -> Option<String> {
        let cell_px = self.canvas.cell_px();
        let zoom = self.canvas.camera.zoom;
        // Заголовки отделов.
        for object in &self.canvas.objects {
            if object.kind != NodeKind::Department {
                continue;
            }
            let r = &object.rect;
            let rect_px = egui::Rect::from_min_size(
                self.canvas.cell_to_screen(r.x, r.y),
                egui::Vec2::new(r.w as f32 * cell_px, r.h as f32 * cell_px),
            );
            let header_h = 80.0 * zoom;
            let header_rect =
                egui::Rect::from_min_size(rect_px.min, egui::Vec2::new(rect_px.width(), header_h));
            if header_rect.contains(screen_pos) {
                return Some(object.id.clone());
            }
        }
        // Заголовки процессов (только верхняя часть карточки).
        for object in &self.canvas.objects {
            if object.kind != NodeKind::Process {
                continue;
            }
            let r = &object.rect;
            let rect_px = self.canvas.cell_to_screen(r.x, r.y);
            let header_h = PROCESS_HEADER_H_PX * zoom;
            let header_rect =
                egui::Rect::from_min_size(rect_px, egui::Vec2::new(512.0 * zoom, header_h));
            if header_rect.contains(screen_pos) {
                return Some(object.id.clone());
            }
        }
        None
    }

    /// Найти заголовок контейнера под заданной экранной позицией.
    /// Включает контейнерные шапки внутри карточек процессов
    /// (у таких записей id = id процесса, direction = направление контейнера).
    pub(crate) fn container_header_under(&self, screen_pos: Pos2) -> Option<(String, ContainerDirection)> {
        for (id, dir, rect) in &self.canvas.container_header_hotspots {
            if rect.contains(screen_pos) {
                let container_dir = match dir {
                    Direction::Request => ContainerDirection::Request,
                    Direction::Result => ContainerDirection::Result,
                };
                return Some((id.clone(), container_dir));
            }
        }
        None
    }

    /// Найти процесс под экранной позицией (динамическая CSS-геометрия).
    pub(crate) fn process_under(&self, pos: Pos2, dynamic: &polygram_core::DynamicState) -> Option<String> {
        let zoom = self.canvas.camera.zoom;
        for object in &self.canvas.objects {
            if object.kind != NodeKind::Process {
                continue;
            }
            let r = &object.rect;
            let card_w = 512.0 * zoom;
            let card_h = polygram_canvas::paint::object::process_total_height(object, dynamic, zoom, card_w);
            let rect_px = egui::Rect::from_min_size(
                self.canvas.cell_to_screen(r.x, r.y),
                egui::Vec2::new(card_w, card_h),
            );
            if rect_px.contains(pos) {
                return Some(object.id.clone());
            }
        }
        None
    }
}

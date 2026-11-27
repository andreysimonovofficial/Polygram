//! Hit-testing canvas: определение объекта, шапки, отдела под экранной позицией.
//! Отвечает за:
//! - `department_covered_by_cover` — прикрыт ли отдел обложкой родителя;
//! - `object_at_any` — любой объект (процесс или отдел) под позицией;
//! - `object_at` — отдел под позицией;
//! - `header_at` — шапка отдела под позицией.

use egui::{Pos2, Rect, Vec2};
use polygram_core::geometry::ObjectGeom;
use polygram_core::types::NodeKind;
use crate::Canvas;

impl Canvas {
    /// Является ли отдел `obj` прикрыт обложкой не наведённого родительского отдела.
    /// Если какой-то departamentos-предок не наведён, его обложка (третий проход `paint`)
    /// перекрывает всё содержимое отдела, включая сам этот отдел — так же, как дочерние
    /// процессы скрываются обложкой не наведённого родителя.
    pub fn department_covered_by_cover(&self, pointer_pos: Option<Pos2>, obj: &ObjectGeom) -> bool {
        let cell_px = self.cell_px();
        let mut parent_id = obj.parent.clone();
        while let Some(pid) = parent_id {
            if let Some(parent) = self.objects.iter().find(|o| o.id == pid) {
                if parent.kind == NodeKind::Department {
                    let pr = &parent.rect;
                    let prect = Rect::from_min_size(
                        self.cell_to_screen(pr.x, pr.y),
                        Vec2::new(pr.w as f32 * cell_px, pr.h as f32 * cell_px),
                    );
                    let p_hovered = pointer_pos.map_or(false, |p| prect.contains(p));
                    if !p_hovered {
                        return true;
                    }
                    parent_id = parent.parent.clone();
                } else {
                    parent_id = parent.parent.clone();
                }
            } else {
                break;
            }
        }
        false
    }

    /// Вернуть id любого объекта (процесса или отдела), под которым находится заданная экранная позиция.
    /// Сначала проверяются процессы (с фиксированной геометрией 256×256px), затем отделы
    /// (возвращается самый вложенный — с наименьшей площадью rect).
    pub fn object_at_any<'a>(&'a self, screen_pos: Pos2, dynamic: &polygram_core::DynamicState) -> Option<&'a str> {
        let cell_px = self.cell_px();
        let zoom = self.camera.zoom;

        // Сначала процессы (динамическая CSS-геометрия).
        for object in &self.objects {
            if object.kind != NodeKind::Process {
                continue;
            }
            let r = &object.rect;
            let card_w = 512.0 * zoom;
            let card_h = crate::paint::object::process_total_height(object, dynamic, zoom, card_w);
            let rect_px = Rect::from_min_size(
                self.cell_to_screen(r.x, r.y),
                Vec2::new(card_w, card_h),
            );
            if rect_px.contains(screen_pos) {
                return Some(&object.id);
            }
        }

        // Затем самый вложенный отдел (наименьшая площадь rect).
        let mut best: Option<(&'a str, f32)> = None;
        for object in &self.objects {
            if object.kind != NodeKind::Department {
                continue;
            }
            let r = &object.rect;
            let rect_px = Rect::from_min_size(
                self.cell_to_screen(r.x, r.y),
                Vec2::new(r.w as f32 * cell_px, r.h as f32 * cell_px),
            );
            if rect_px.contains(screen_pos) {
                let area = rect_px.area();
                if best.map_or(true, |(_, a)| area < a) {
                    best = Some((&object.id, area));
                }
            }
        }
        best.map(|(id, _)| id)
    }

    /// Вернуть id отдела, под которым находится заданная экранная позиция.
    pub fn object_at<'a>(&'a self, screen_pos: Pos2, cell_px: f32) -> Option<&'a str> {
        for object in &self.objects {
            if object.kind != NodeKind::Department {
                continue;
            }
            let r = &object.rect;
            let rect_px = Rect::from_min_size(
                egui::pos2(r.x as f32 * cell_px, r.y as f32 * cell_px),
                egui::Vec2::new(r.w as f32 * cell_px, r.h as f32 * cell_px),
            );
            if rect_px.contains(screen_pos) {
                return Some(&object.id);
            }
        }
        None
    }

    /// Вернуть id отдела, чья шапка содержит заданную экранную позицию.
    pub fn header_at<'a>(&'a self, screen_pos: Pos2, cell_px: f32) -> Option<&'a str> {
        for object in &self.objects {
            if object.kind != NodeKind::Department {
                continue;
            }
            let r = &object.rect;
            let header_h = cell_px * 1.0;
            let header_rect = Rect::from_min_size(
                egui::pos2(r.x as f32 * cell_px, r.y as f32 * cell_px),
                egui::Vec2::new(r.w as f32 * cell_px, header_h),
            );
            if header_rect.contains(screen_pos) {
                return Some(&object.id);
            }
        }
        None
    }
}

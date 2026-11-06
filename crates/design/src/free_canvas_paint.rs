//! Отрисовка фигур, проверка попадания и удаление выделенного.

use crate::design_format::{DrawTool, FreeShape, TextAlign};
use crate::free_canvas::FreeCanvas;
use egui::{Pos2, Rect, Rounding, Shape, Stroke, Vec2};
use egui::Color32;
use egui::FontId;

/// Расстояние от точки до отрезка.
pub fn dist_point_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let ap = p - a;
    let mut t = (ap.x * ab.x + ap.y * ab.y) / (ab.x * ab.x + ab.y * ab.y).max(1e-6);
    t = t.clamp(0.0, 1.0);
    let closest = a + ab * t;
    (p - closest).length()
}

impl FreeCanvas {
    /// Проверка попадания курсора в фигуру (мировые координаты).
    pub(crate) fn hit_shape(&self, pos: Pos2) -> Option<usize> {
        for (i, shape) in self.doc.shapes.iter().enumerate().rev() {
            let rect = Rect::from_min_size(
                Pos2::new(shape.x, shape.y),
                Vec2::new(shape.w.max(1.0), shape.h.max(1.0)),
            );
            match shape.tool {
                DrawTool::Line => {
                    let p1 = Pos2::new(shape.x, shape.y);
                    let p2 = Pos2::new(shape.x + shape.w, shape.y + shape.h);
                    if dist_point_segment(pos, p1, p2) < 6.0 {
                        return Some(i);
                    }
                }
                _ => {
                    if rect.contains(pos) {
                        return Some(i);
                    }
                }
            }
        }
        None
    }

    pub(crate) fn delete_selected(&mut self) {
        if let Some(idx) = self.selected.take() {
            if idx < self.doc.shapes.len() {
                self.doc.shapes.remove(idx);
            }
        }
    }

    /// Нарисовать одну фигуру.
    pub(crate) fn paint_shape(
        &self,
        painter: &egui::Painter,
        shape: &FreeShape,
        rect: Rect,
        _is_selected: bool,
    ) {
        let fill = shape
            .fill
            .map(|c| Color32::from_rgb(c.r, c.g, c.b))
            .unwrap_or(Color32::TRANSPARENT);
        let stroke_color = Color32::from_rgb(shape.stroke.r, shape.stroke.g, shape.stroke.b);
        let stroke = Stroke::new(shape.stroke_width.max(0.0), stroke_color);

        let rounding = if shape.corner_tl == shape.corner_tr
            && shape.corner_tr == shape.corner_bl
            && shape.corner_bl == shape.corner_br
        {
            Rounding::same(shape.corner_tl)
        } else {
            Rounding {
                nw: shape.corner_tl,
                ne: shape.corner_tr,
                sw: shape.corner_bl,
                se: shape.corner_br,
            }
        };

        match shape.tool {
            DrawTool::Select => {}
            DrawTool::Rect | DrawTool::Text => {
                painter.rect_filled(rect, rounding, fill);
                if shape.stroke_width > 0.0 {
                    painter.rect_stroke(rect, rounding, stroke);
                }
            }
            DrawTool::Ellipse => {
                let center = rect.center();
                let radii = Vec2::new(rect.width() / 2.0, rect.height() / 2.0);
                painter.add(Shape::ellipse_filled(center, radii, fill));
                if shape.stroke_width > 0.0 {
                    painter.add(Shape::ellipse_stroke(center, radii, stroke));
                }
            }
            DrawTool::Line => {
                let p1 = Pos2::new(rect.min.x, rect.min.y);
                let p2 = Pos2::new(rect.max.x, rect.max.y);
                painter.line_segment([p1, p2], stroke);
            }
        }

        // Текст
        if !shape.text.is_empty() {
            let text_color =
                Color32::from_rgb(shape.text_color.r, shape.text_color.g, shape.text_color.b);
            let font = FontId::proportional(shape.font_size.max(6.0));
            let galley = painter.layout_no_wrap(shape.text.clone(), font, text_color);
            let pos = match shape.text_align {
                TextAlign::Left => rect.min + Vec2::new(8.0, rect.height() / 2.0),
                TextAlign::Center => rect.center() - Vec2::new(galley.size().x / 2.0, galley.size().y / 2.0),
                TextAlign::Right => rect.max - Vec2::new(galley.size().x + 8.0, rect.height() / 2.0),
            };
            painter.galley(pos, galley, text_color);
        }
    }
}

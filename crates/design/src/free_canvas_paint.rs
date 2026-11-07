//! Отрисовка фигур, проверка попадания и удаление выделенного.

use crate::design_format::{DrawTool, FreeShape, ShapeState, StrokePosition, TextAlign, TextAlignV, font_key};
use crate::free_canvas::FreeCanvas;
use egui::{Pos2, Rect, Rounding, Shape, Stroke, Vec2};
use egui::Color32;

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

    /// Нарисовать одну фигуру с учётом визуального состояния (прототип).
    pub(crate) fn paint_shape(
        &self,
        painter: &egui::Painter,
        shape: &FreeShape,
        rect: Rect,
        _is_selected: bool,
        state: ShapeState,
        zoom: f32,
    ) {
        let (fill, stroke_color, stroke_width) = Self::resolve_style(shape, state);
        // Толщина обводки масштабируется с зумом, чтобы при приближении она была толще,
        // как на снимке экрана.
        let stroke_width = stroke_width.max(0.0) * zoom;
        let stroke = Stroke::new(stroke_width, stroke_color);

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
                if stroke_width > 0.0 {
                    let stroke_rect = Self::stroke_rect(rect, stroke_width, shape.stroke_position);
                    painter.rect_stroke(stroke_rect, rounding, stroke);
                }
            }
            DrawTool::Ellipse => {
                let center = rect.center();
                let fill_radii = Vec2::new(rect.width() / 2.0, rect.height() / 2.0);
                painter.add(Shape::ellipse_filled(center, fill_radii, fill));
                if stroke_width > 0.0 {
                    let stroke_radii = Self::stroke_radii(fill_radii, stroke_width, shape.stroke_position);
                    painter.add(Shape::ellipse_stroke(center, stroke_radii, stroke));
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
            let text_color = match state {
                ShapeState::Hover => shape
                    .states
                    .hover
                    .as_ref()
                    .and_then(|s| s.fill)
                    .map(|c| Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a))
                    .unwrap_or(Color32::from_rgba_unmultiplied(shape.text_color.r, shape.text_color.g, shape.text_color.b, shape.text_color.a)),
                ShapeState::Pressed => shape
                    .states
                    .pressed
                    .as_ref()
                    .and_then(|s| s.fill)
                    .map(|c| Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a))
                    .unwrap_or(Color32::from_rgba_unmultiplied(shape.text_color.r, shape.text_color.g, shape.text_color.b, shape.text_color.a)),
                ShapeState::Normal => Color32::from_rgba_unmultiplied(shape.text_color.r, shape.text_color.g, shape.text_color.b, shape.text_color.a),
            };
            let font_id = egui::FontId::new(
                shape.font_size.max(6.0) * zoom,
                egui::FontFamily::Name(font_key(shape.font_family, shape.font_weight).into()),
            );
            let galley = painter.layout_no_wrap(shape.text.clone(), font_id, text_color);
            let text_w = galley.size().x;
            let text_h = galley.size().y;
            let x = match shape.text_align {
                TextAlign::Left => rect.min.x,
                TextAlign::Center => rect.center().x - text_w / 2.0,
                TextAlign::Right => rect.max.x - text_w,
            };
            let y = match shape.text_align_v {
                TextAlignV::Top => rect.min.y,
                TextAlignV::Middle => rect.center().y - text_h / 2.0,
                TextAlignV::Bottom => rect.max.y - text_h,
            };
            painter.galley(Pos2::new(x, y), galley, text_color);
        }
    }

    /// Разрешить цвета и толщину обводки для данного состояния.
    fn resolve_style(shape: &FreeShape, state: ShapeState) -> (Color32, Color32, f32) {
        let base_fill = shape
            .fill
            .map(|c| Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a))
            .unwrap_or(Color32::TRANSPARENT);
        let base_stroke = Color32::from_rgba_unmultiplied(shape.stroke.r, shape.stroke.g, shape.stroke.b, shape.stroke.a);
        let base_width = shape.stroke_width.max(0.0);

        match state {
            ShapeState::Normal => (base_fill, base_stroke, base_width),
            ShapeState::Hover => {
                let s = shape.states.hover.as_ref();
                let fill = s.and_then(|st| st.fill).map_or(base_fill, |c| Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a));
                let stroke = s.and_then(|st| st.stroke).map_or(base_stroke, |c| Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a));
                let width = s.and_then(|st| st.stroke_width).unwrap_or(base_width);
                (fill, stroke, width)
            }
            ShapeState::Pressed => {
                let s = shape.states.pressed.as_ref();
                let fill = s.and_then(|st| st.fill).map_or(base_fill, |c| Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a));
                let stroke = s.and_then(|st| st.stroke).map_or(base_stroke, |c| Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a));
                let width = s.and_then(|st| st.stroke_width).unwrap_or(base_width);
                (fill, stroke, width)
            }
        }
    }

    /// Скорректировать прямоугольник под позицию обводки.
    /// egui рисует `rect_stroke` полностью снаружи края переданного rect,
    /// поэтому чтоб обводка лежала внутри/по центру/снаружи границы фигуры,
    /// сдвигаем край rect перед рисованием обводки.
    fn stroke_rect(rect: Rect, stroke_width: f32, position: StrokePosition) -> Rect {
        let max_shrink = (rect.width().min(rect.height()) / 2.0).max(0.0);
        match position {
            StrokePosition::Outside => rect,
            StrokePosition::Center => rect.shrink((stroke_width / 2.0).min(max_shrink)),
            StrokePosition::Inside => rect.shrink(stroke_width.min(max_shrink)),
        }
    }

    /// Скорректировать радиусы эллипса под позицию обводки.
    fn stroke_radii(radii: Vec2, stroke_width: f32, position: StrokePosition) -> Vec2 {
        let max_shrink = radii.x.min(radii.y).max(0.0);
        match position {
            StrokePosition::Outside => radii,
            StrokePosition::Center => radii - Vec2::splat((stroke_width / 2.0).min(max_shrink)),
            StrokePosition::Inside => radii - Vec2::splat(stroke_width.min(max_shrink)),
        }
    }
}

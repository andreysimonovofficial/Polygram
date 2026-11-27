//! Поиск свободной клетки внутри отдела и целевого объекта под клеткой.

use crate::geometry::{Geometry, ObjectGeom};
use crate::layout::Rect;
use crate::types::{ContainerDirection, NodeKind};

/// Найти свободную клетку внутри отдела для объекта размером w×h клеток.
pub fn find_free_cell_in_dept_domain(
    geometry: &Geometry,
    dept_id: &str,
    w: i32,
    h: i32,
    exclude_id: &str,
) -> Option<(i32, i32)> {
    let dept = geometry.objects.iter().find(|o| o.id == dept_id && o.kind == NodeKind::Department)?;
    let inner_x0 = dept.rect.x + 1;
    let inner_y0 = dept.rect.y + 2;
    let inner_x1 = dept.rect.x + dept.rect.w - 1;
    let inner_y1 = dept.rect.y + dept.rect.h - 1;
    if inner_x1 < inner_x0 + w || inner_y1 < inner_y0 + h {
        return None;
    }
    for y in inner_y0..=inner_y1.saturating_sub(h) {
        for x in inner_x0..=inner_x1.saturating_sub(w) {
            let r = Rect::new(x, y, w, h);
            let free = geometry.objects.iter().all(|o| {
                o.id == exclude_id || o.id == dept_id || !o.rect.overlaps(&r)
            });
            if free {
                return Some((x, y));
            }
        }
    }
    None
}

/// Найти объект-цель под клеткой (исключая dragged_id).
pub fn target_object_at<'a>(
    geometry: &'a Geometry,
    cx: i32,
    cy: i32,
    exclude_id: &str,
) -> Option<&'a ObjectGeom> {
    geometry.objects.iter().find(|o| {
        o.id != exclude_id
            && o.rect.x <= cx && cx < o.rect.x + o.rect.w
            && o.rect.y <= cy && cy < o.rect.y + o.rect.h
    })
}

/// Направление, противоположное заданному.
pub fn opposite_dir(dir: ContainerDirection) -> ContainerDirection {
    match dir {
        ContainerDirection::Request => ContainerDirection::Result,
        ContainerDirection::Result => ContainerDirection::Request,
    }
}

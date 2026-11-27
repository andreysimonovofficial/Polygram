//! Поиск свободной клетки внутри отдела для размещения объекта.

use crate::app::PolygramApp;
use polygram_core::layout::Rect as CoreRect;
use polygram_core::types::NodeKind;

/// Найти свободную клетку внутри отдела для размещения объекта размером w×h клеток.
/// Поиск ограничивается внутренней областью отдела (под шапкой, между детьми).
pub(crate) fn find_free_cell_in_dept(
    app: &PolygramApp,
    dept_id: &str,
    w: i32,
    h: i32,
    exclude_id: &str,
) -> Option<(i32, i32)> {
    let dept = app.canvas.objects.iter().find(|o| o.id == dept_id && o.kind == NodeKind::Department)?;
    // Внутренняя область отдела: отступ 1 клетка по бокам, шапка занимает верхнюю строку.
    let inner_x0 = dept.rect.x + 1;
    let inner_y0 = dept.rect.y + 2;
    let inner_x1 = dept.rect.x + dept.rect.w - 1;
    let inner_y1 = dept.rect.y + dept.rect.h - 1;
    if inner_x1 < inner_x0 + w || inner_y1 < inner_y0 + h {
        return None;
    }
    // Сначала пробуем клетки рядом с верхним левым углу внутренней области, по строкам.
    for y in inner_y0..=inner_y1.saturating_sub(h) {
        for x in inner_x0..=inner_x1.saturating_sub(w) {
            let r = CoreRect::new(x, y, w, h);
            let free = app.canvas.objects.iter().all(|o| {
                o.id == exclude_id || o.id == dept_id || !o.rect.overlaps(&r)
            });
            if free {
                return Some((x, y));
            }
        }
    }
    None
}

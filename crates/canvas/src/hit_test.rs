use egui::Pos2;
use polygram_core::layout::Rect as CoreRect;
use polygram_core::Geometry;
use polygram_core::types::NodeKind;

fn rect_to_screen(
    object: &polygram_core::ObjectGeom,
    cell_size: f32,
    camera_offset: egui::Vec2,
    camera_zoom: f32,
) -> egui::Rect {
    let r = CoreRect::new(object.rect.x, object.rect.y, object.rect.w, object.rect.h);
    egui::Rect::from_min_size(
        world_to_screen(Pos2::new(r.x as f32, r.y as f32), cell_size, camera_offset, camera_zoom),
        egui::Vec2::new(
            r.w as f32 * cell_size * camera_zoom,
            r.h as f32 * cell_size * camera_zoom,
        ),
    )
}

/// Итерация по всем объектам (сначала обычные, потом отделы) с остановкой на первом совпадении.
fn find_object_with<'a, F>(
    geometry: &'a Geometry,
    camera_offset: egui::Vec2,
    camera_zoom: f32,
    cell_size: f32,
    mut pred: F,
) -> Option<&'a str>
where
    F: FnMut(&polygram_core::ObjectGeom, egui::Rect) -> bool,
{
    for object in &geometry.objects {
        if object.kind == NodeKind::Department {
            continue;
        }
        let rect_px = rect_to_screen(object, cell_size, camera_offset, camera_zoom);
        if pred(object, rect_px) {
            return Some(&object.id);
        }
    }
    for object in &geometry.objects {
        if object.kind != NodeKind::Department {
            continue;
        }
        let rect_px = rect_to_screen(object, cell_size, camera_offset, camera_zoom);
        if pred(object, rect_px) {
            return Some(&object.id);
        }
    }
    None
}

/// Определить объект под заданной экранной позицией.
pub fn object_at<'a>(
    geometry: &'a Geometry,
    screen_pos: Pos2,
    camera_offset: egui::Vec2,
    camera_zoom: f32,
    cell_size: f32,
) -> Option<&'a str> {
    find_object_with(geometry, camera_offset, camera_zoom, cell_size, |_, rect_px| {
        rect_px.contains(screen_pos)
    })
}

/// Вернуть id самого вложенного отдела, содержащего заданную экранную позицию.
pub fn department_at<'a>(
    geometry: &'a Geometry,
    screen_pos: Pos2,
    camera_offset: egui::Vec2,
    camera_zoom: f32,
    cell_size: f32,
) -> Option<&'a str> {
    let cell_px = cell_size * camera_zoom;
    let mut best: Option<(&'a str, f32)> = None;
    for object in &geometry.objects {
        if object.kind != NodeKind::Department {
            continue;
        }
        let rect_px = rect_to_screen(object, cell_size, camera_offset, camera_zoom);
        if rect_px.contains(screen_pos) {
            let area = rect_px.area();
            if best.map_or(true, |(_, a)| area < a) {
                best = Some((&object.id, area));
            }
        }
    }
    best.map(|(id, _)| id)
}

/// Вернуть id объекта, если экранная позиция попадает в его шапку.
pub fn header_at<'a>(
    geometry: &'a Geometry,
    screen_pos: Pos2,
    camera_offset: egui::Vec2,
    camera_zoom: f32,
    cell_size: f32,
) -> Option<&'a str> {
    let cell_px = cell_size * camera_zoom;
    find_object_with(geometry, camera_offset, camera_zoom, cell_size, |object, rect_px| {
        let header_h = if object.kind == NodeKind::Department {
            cell_px * 1.2
        } else {
            cell_px
        };
        let header_rect =
            egui::Rect::from_min_size(rect_px.min, egui::Vec2::new(rect_px.width(), header_h));
        header_rect.contains(screen_pos)
    })
}

fn world_to_screen(p: Pos2, cell_size: f32, offset: egui::Vec2, zoom: f32) -> Pos2 {
    Pos2::new(
        (p.x * cell_size + offset.x) * zoom,
        (p.y * cell_size + offset.y) * zoom,
    )
}

/// Возвращает объект, чей rect содержит клетку (cx, cy), исключая exclude_id.
pub fn object_at_cell<'a>(
    geometry: &'a Geometry,
    cx: i32,
    cy: i32,
    exclude_id: &str,
) -> Option<&'a str> {
    for object in &geometry.objects {
        if object.id == exclude_id {
            continue;
        }
        if object.rect.x <= cx
            && cx < object.rect.x + object.rect.w
            && object.rect.y <= cy
            && cy < object.rect.y + object.rect.h
        {
            return Some(&object.id);
        }
    }
    None
}

/// Свободна ли клетка (cx, cy) (нет объектов, кроме exclude_id).
pub fn is_cell_free(geometry: &Geometry, cx: i32, cy: i32, exclude_id: &str) -> bool {
    object_at_cell(geometry, cx, cy, exclude_id).is_none()
}

/// Вернута ли клетка внутри отдела dept_id.
pub fn inside_dept(geometry: &Geometry, dept_id: &str, cx: i32, cy: i32) -> bool {
    geometry.objects.iter().find(|o| o.id == dept_id).map_or(false, |d| {
        d.rect.x <= cx && cx < d.rect.x + d.rect.w && d.rect.y <= cy && cy < d.rect.y + d.rect.h
    })
}

/// Свободен ли rect (без учета exclude_id) — нет пересечений с другими объектами.
pub fn rect_is_free(
    geometry: &Geometry,
    rect: &polygram_core::layout::Rect,
    exclude_id: &str,
) -> bool {
    for object in &geometry.objects {
        if object.id == exclude_id {
            continue;
        }
        if object.rect.overlaps(rect) {
            return false;
        }
    }
    true
}

/// Свободная клетка для призрачной копии на расстоянии `offset` клеток от края объекта obj.
/// Пытаемся 4 стороны (справа, слева, снизу, сверху). Если ни одна не свободна —
/// спиральный поиск от центра объекта, начиная снаружи от его границ.
/// Если dept_id задан — ищем только внутри этого отдела.
pub fn find_ghost_cell(
    geometry: &Geometry,
    obj: &polygram_core::ObjectGeom,
    ghost_w: i32,
    ghost_h: i32,
    offset: i32,
    exclude_id: &str,
    dept_id: Option<&str>,
) -> Option<(i32, i32)> {
    let r_obj = &obj.rect;
    let inside = |cx: i32, cy: i32| dept_id.map_or(true, |d| inside_dept(geometry, d, cx, cy));

    // 4 позиции со смещением offset от краёв объекта.
    let candidates: [(i32, i32); 4] = [
        (r_obj.x + r_obj.w + offset, r_obj.y),           // справа
        (r_obj.x - offset - ghost_w, r_obj.y),            // слева
        (r_obj.x, r_obj.y + r_obj.h + offset),            // снизу
        (r_obj.x, r_obj.y - offset - ghost_h),            // сверху
    ];
    for (cx, cy) in &candidates {
        if !inside(*cx, *cy) {
            continue;
        }
        let r_ghost = polygram_core::layout::Rect::new(*cx, *cy, ghost_w, ghost_h);
        if rect_is_free(geometry, &r_ghost, exclude_id) {
            return Some((*cx, *cy));
        }
    }

    // Fallback: спиральный поиск от центра объекта, начиная снаружи от его границ.
    let cx0 = r_obj.x + r_obj.w / 2;
    let cy0 = r_obj.y + r_obj.h / 2;
    let min_r = (r_obj.w.max(r_obj.h) / 2) + offset;
    for r in min_r..=min_r + 12 {
        for dx in -r..=r {
            for dy in -r..=r {
                if dx.abs() != r && dy.abs() != r {
                    continue;
                }
                let cx = cx0 + dx;
                let cy = cy0 + dy;
                if !inside(cx, cy) {
                    continue;
                }
                let r_ghost = polygram_core::layout::Rect::new(cx, cy, ghost_w, ghost_h);
                if rect_is_free(geometry, &r_ghost, exclude_id) {
                    return Some((cx, cy));
                }
            }
        }
    }
    None
}

/// Для отдела: свободная клетка рядом с содержащимися в нём процессами на расстоянии `offset`.
/// Поиск не ограничивается текущими границами dept.rect — отдел растянется под
/// найденную клетку через fit_departments_to_children на этапе рендера.
pub fn find_ghost_cell_in_dept(
    geometry: &Geometry,
    dept_id: &str,
    ghost_w: i32,
    ghost_h: i32,
    offset: i32,
    exclude_id: &str,
) -> Option<(i32, i32)> {
    let dept = geometry.objects.iter().find(|o| o.id == dept_id)?;
    let children: Vec<&polygram_core::ObjectGeom> = geometry
        .objects
        .iter()
        .filter(|o| o.parent.as_deref() == Some(dept_id) && o.id != exclude_id)
        .collect();

    // Для каждого дочернего процесса пробуем 4 позиции со смещением offset от его краёв.
    for child in &children {
        let r_child = &child.rect;
        let candidates: [(i32, i32); 4] = [
            (r_child.x + r_child.w + offset, r_child.y),
            (r_child.x - offset - ghost_w, r_child.y),
            (r_child.x, r_child.y + r_child.h + offset),
            (r_child.x, r_child.y - offset - ghost_h),
        ];
        for (cx, cy) in candidates {
            let r_ghost = polygram_core::layout::Rect::new(cx, cy, ghost_w, ghost_h);
            if rect_is_free(geometry, &r_ghost, exclude_id) {
                return Some((cx, cy));
            }
        }
    }

    // Fallback: спиральный поиск от центра отдела наружу (без ограничения dept.rect).
    let cx0 = dept.rect.x + dept.rect.w / 2;
    let cy0 = dept.rect.y + dept.rect.h / 2;
    let min_r = offset;
    for r in min_r..=min_r + 24 {
        for dx in -r..=r {
            for dy in -r..=r {
                if dx.abs() != r && dy.abs() != r {
                    continue;
                }
                let cx = cx0 + dx;
                let cy = cy0 + dy;
                let r_ghost = polygram_core::layout::Rect::new(cx, cy, ghost_w, ghost_h);
                if rect_is_free(geometry, &r_ghost, exclude_id) {
                    return Some((cx, cy));
                }
            }
        }
    }
    None
}

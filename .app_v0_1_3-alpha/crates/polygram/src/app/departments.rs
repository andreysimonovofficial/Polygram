use crate::app::PolygramApp;
use polygram_core::{geometry::{Geometry, ObjectGeom}, types::NodeKind};

/// Построить геометрию с учётом текущего перетаскивания, чтобы отделы
/// растягивались под содержимое «на лету» ещё до подтверждения сервером.
/// Используется для hit-testing и отрисовки.
pub fn geometry_with_predictions(
    objects: &[ObjectGeom],
    _drag_object: Option<&str>,
    _drag_over: Option<&str>,
) -> Geometry {
    let mut geometry = Geometry {
        rev: 0,
        objects: objects.to_vec(),
        edges: Vec::new(),
    };
    fit_departments_to_children(&mut geometry, None);
    geometry
}

/// Высота шапки отдела в клетках (80px / 16px на клетку = 5).
const DEPT_HEADER_CELLS: i32 = 5;

/// Вычислить rect каждого отдела как обтягивающую рамку его прямых детей.
/// Отдел не имеет фиксированного размера: рамка следует за содержимым.
/// Повторный обход нужен, чтобы вложенные отделы уже имели актуальный размер,
/// когда считается родительский отдел. Пустой отдел сохраняет текущий размер
/// (минимальное место под первого ребёнка).
/// `exclude_id` — объект, который не учитывается при построении рамки отделов
/// (перетаскиваемая карточка, чтобы отдел не «захватывал» её на лету).
pub fn fit_departments_to_children(geometry: &mut Geometry, exclude_id: Option<&str>) {
    use polygram_core::layout::Rect;
    let mut children: std::collections::HashMap<String, Vec<usize>> = std::collections::HashMap::new();
    for (i, o) in geometry.objects.iter().enumerate() {
        if exclude_id.is_some_and(|e| e == o.id) {
            continue;
        }
        if let Some(p) = &o.parent {
            children.entry(p.clone()).or_default().push(i);
        }
    }
    for _ in 0..8 {
        let mut changed = false;
        for (dept_id, child_indices) in &children {
            if child_indices.is_empty() {
                continue;
            }
            let dept_idx = match geometry.objects.iter().position(|o| o.id == *dept_id) {
                Some(i) => i,
                None => continue,
            };
            // Только отделы растягиваются под детей. Процессы и контейнеры
            // сохраняют свой rect из layout.
            if geometry.objects[dept_idx].kind != NodeKind::Department {
                continue;
            }
            let mut min_x = i32::MAX;
            let mut min_y = i32::MAX;
            let mut max_x = i32::MIN;
            let mut max_y = i32::MIN;
            for &ci in child_indices {
                let c = &geometry.objects[ci];
                min_x = min_x.min(c.rect.x);
                min_y = min_y.min(c.rect.y);
                max_x = max_x.max(c.rect.x + c.rect.w);
                max_y = max_y.max(c.rect.y + c.rect.h);
            }
            let new_x = min_x - 1;
            let new_y = min_y - DEPT_HEADER_CELLS;
            let new_w = (max_x + 1) - new_x;
            let new_h = (max_y + 1) - new_y;
            let new_rect = Rect::new(new_x, new_y, new_w, new_h);
            let dept = &mut geometry.objects[dept_idx];
            if dept.rect != new_rect {
                dept.rect = new_rect;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
}

/// Растянуть рамку отдела под содержащийся в нём перетаскиваемый объект «на лету».
/// Членство определяется пересечением с текущей рамкой отдела: объект, попадающий
/// под курсор при старте перетаскивания, считается ребёнком отдела. Тянет
/// рекурсивно родительские отделы по цепочке вверх.
#[allow(dead_code)]
pub fn stretch_department_for_drag(geometry: &mut Geometry, dept_id: &str) {
    use polygram_core::layout::Rect;
    let mut current = dept_id.to_owned();
    let mut visited: Vec<String> = Vec::new();
    for _ in 0..8 {
        if visited.contains(&current) {
            break;
        }
        visited.push(current.clone());
        let dept_idx = match geometry.objects.iter().position(|o| o.id == current) {
            Some(i) => i,
            None => break,
        };
        let dept = &geometry.objects[dept_idx];
        let mut min_x = dept.rect.x;
        let mut min_y = dept.rect.y;
        let mut max_x = dept.rect.x + dept.rect.w;
        let mut max_y = dept.rect.y + dept.rect.h;
        for obj in &geometry.objects {
            if obj.id == current {
                continue;
            }
            // Прямой ребёнок отдела по иерархии parent — только такие объекты
            // участвуют в построении рамки отдела.
            if obj.parent.as_ref() != Some(&current) {
                continue;
            }
            min_x = min_x.min(obj.rect.x);
            min_y = min_y.min(obj.rect.y);
            max_x = max_x.max(obj.rect.x + obj.rect.w);
            max_y = max_y.max(obj.rect.y + obj.rect.h);
        }
            let new_x = min_x - 1;
            let new_y = min_y - DEPT_HEADER_CELLS;
            let new_w = (max_x + 1) - new_x;
            let new_h = (max_y + 1) - new_y;
        let new_rect = Rect::new(new_x, new_y, new_w, new_h);
        if new_rect == dept.rect {
            break;
        }
        geometry.objects[dept_idx].rect = new_rect;
        // Переходим к родительскому отделу через поле parent.
        current = match geometry.objects.iter().find(|o| o.id == current)
            .and_then(|o| o.parent.clone())
        {
            Some(p) => p,
            None => break,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use polygram_core::{layout::Rect, types::NodeKind};

    fn dept(id: &str, rect: Rect) -> ObjectGeom {
        ObjectGeom {
            ix: 0,
            id: id.to_owned(),
            name: id.to_owned(),
            kind: NodeKind::Department,
            rect,
            direction: None,
            requests: Vec::new(),
            results: Vec::new(),
            parent: None,
        }
    }

    fn proc(id: &str, rect: Rect, parent: &str) -> ObjectGeom {
        ObjectGeom {
            ix: 0,
            id: id.to_owned(),
            name: id.to_owned(),
            kind: NodeKind::Process,
            rect,
            direction: None,
            requests: Vec::new(),
            results: Vec::new(),
            parent: Some(parent.to_owned()),
        }
    }

    #[test]
    fn department_stretches_to_direct_children() {
        let mut geometry = Geometry {
            rev: 0,
            objects: vec![
                dept("d1", Rect::new(0, 0, 30, 30)),
                proc("p1", Rect::new(5, 5, 16, 16), "d1"),
                proc("p2", Rect::new(30, 20, 16, 16), "d1"),
            ],
            edges: Vec::new(),
        };
        fit_departments_to_children(&mut geometry, None);
        let d = geometry.objects.iter().find(|o| o.id == "d1").unwrap();
        // min_x=5-1=4, min_y=5-5=0, max_x=30+16+1=47, max_y=20+16+1=37
        // w = 47-4=43, h = 37-0=37
        assert_eq!(d.rect, Rect::new(4, 0, 43, 37));
    }

    #[test]
    fn nested_departments_stretch_bottom_up() {
        let mut geometry = Geometry {
            rev: 0,
            objects: vec![
                dept("root", Rect::new(0, 0, 30, 30)),
                dept("child", Rect::new(0, 0, 30, 30)),
                proc("p1", Rect::new(5, 5, 16, 16), "child"),
            ],
            edges: Vec::new(),
        };
        // Set parents explicitly after construction.
        geometry.objects[1].parent = Some("root".to_owned());
        geometry.objects[2].parent = Some("child".to_owned());
        fit_departments_to_children(&mut geometry, None);
        let child = geometry.objects.iter().find(|o| o.id == "child").unwrap();
        // p1 at (5,5,16,16): min_x=5-1=4, min_y=5-5=0, max_x=21+1=22, max_y=21+1=22
        assert_eq!(child.rect, Rect::new(4, 0, 18, 22));
        let root = geometry.objects.iter().find(|o| o.id == "root").unwrap();
        // child rect now 4,0,18,22 -> root bounds: min_x=4, min_y=0, max_x=22, max_y=22
        // new_x=4-1=3, new_y=0-5=-5, new_w=(22+1)-3=20, new_h=(22+1)-(-5)=28
        assert_eq!(root.rect, Rect::new(3, -5, 20, 28));
    }

    #[test]
    fn exclude_id_keeps_department_from_capturing_dragged_object() {
        let mut geometry = Geometry {
            rev: 0,
            objects: vec![
                dept("d1", Rect::new(0, 0, 30, 30)),
                proc("p1", Rect::new(5, 5, 16, 16), "d1"),
                proc("dragged", Rect::new(100, 100, 16, 16), "d1"),
            ],
            edges: Vec::new(),
        };
        fit_departments_to_children(&mut geometry, Some("dragged"));
        let d = geometry.objects.iter().find(|o| o.id == "d1").unwrap();
        // dragged excluded -> only p1 counts: min_x=5-1=4, min_y=5-5=0, max_x=21+1=22, max_y=21+1=22
        assert_eq!(d.rect, Rect::new(4, 0, 18, 22));
    }
}

impl PolygramApp {
    /// Геометрия с учётом текущего перетаскивания. Отделы пересчитываются под
    /// содержимое (fit_departments_to_children) на каждом кадре, поэтому клик и
    /// наведение попадают в тот же объект даже во время драга.
    pub fn geometry_with_predictions(&self) -> Geometry {
        crate::app::departments::geometry_with_predictions(
            &self.canvas.objects,
            self.drag_object.as_deref(),
            self.drag_over.as_deref(),
        )
    }
}

use crate::app::PolygramApp;
use polygram_core::{geometry::Geometry, types::NodeKind, Published, Command};
use crate::app::drag::DragState;
use std::time::{SystemTime, UNIX_EPOCH};

/// Построить геометрию с учётом неотправленных на сервер предсказаний позиций (drag).
/// Используется для hit-testing, чтобы клик по визуальной позиции объекта попадал
/// в тот же объект, даже если позиция ещё не подтверждена сервером.
pub fn geometry_with_predictions(
    published: &Published,
    drag: &DragState,
) -> Geometry {
    let mut geometry = published.geometry.clone();
    for obj in &mut geometry.objects {
        if let Some((x, y)) = drag.predicted_positions.get(&obj.id) {
            obj.rect.x = *x;
            obj.rect.y = *y;
        }
    }
    // Если нависаем над отделом и есть призрачная копия — временно считаем объект
    // ребёнком отдела в позиции копии, чтобы fit_departments_to_children растянул
    // отдел и все родительские отделы, охватывая копию ещё до события Drop.
    // Реальная иерархия меняется только на сервере по команде SetObjectParent при Drop.
    let dragged_id = drag.drag_object().map(|s| s.to_string()).unwrap_or_default();
    // Рамка отдела всегда обтягивает всех прямых детей, включая перетаскиваемый
    // объект — так при выводе карточки из отдела рамка тянется за ней (DoD п.3),
    // а при наведении на отдел тянется под призрачную копию.
    let mut temp_reparent: Option<(String, Option<String>)> = None;
    if let Some(dept_id) = &drag.drag_over {
        if geometry.objects.iter().any(|o| o.id == *dept_id && o.kind == NodeKind::Department) {
            if let Some((gx, gy)) = drag.drag_ghost {
                if let Some(obj) = geometry.objects.iter_mut().find(|o| o.id == dragged_id) {
                    temp_reparent = Some((obj.id.clone(), obj.parent.clone()));
                    obj.parent = Some(dept_id.clone());
                    obj.rect.x = gx;
                    obj.rect.y = gy;
                }
            }
        }
    }
    fit_departments_to_children(&mut geometry, None);
    // Вернуть родителя перетаскиваемого объекта: реальная иерархия меняется
    // только на сервере по команде SetObjectParent при Drop.
    if let Some((id, parent)) = temp_reparent {
        if let Some(obj) = geometry.objects.iter_mut().find(|o| o.id == id) {
            obj.parent = parent;
        }
    }
    geometry
}

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
            let new_y = min_y - 2;
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
        let new_y = min_y - 2;
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

impl PolygramApp {
    /// Сгруппировать выделенные объекты в новый отдел.
    /// Вычисляет ограничивающий прямоугольник выделения, создаёт отдел,
    /// размещает его вокруг объектов и переводит каждого выбранного объекта в отдел.
    pub fn group_selected_into_department(&mut self, published: &Published) {
        if self.selected_objects.len() < 2 {
            return;
        }
        let geometry = &published.geometry;
        let mut selected_ids: Vec<String> = Vec::new();
        for id in &self.selected_objects {
            if geometry.objects.iter().any(|o| o.id == *id) {
                selected_ids.push(id.clone());
            }
        }
        if selected_ids.is_empty() {
            return;
        }

        let dept_id = format!("dept_{}", SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0));

        let mut batch: Vec<Command> = Vec::new();
        batch.push(Command::AddObject {
            id: Some(dept_id.clone()),
            name: "Отдел".to_owned(),
            kind: NodeKind::Department,
            at: None,
            parent: None,
            author: None,
        });
        for obj_id in &selected_ids {
            batch.push(Command::SetObjectParent {
                object: obj_id.clone(),
                parent: Some(dept_id.clone()),
            });
        }

        self.apply_batch(batch);
        self.selected_objects.clear();
    }
}

//! Canvas Polygram: background, dot grid, and objects (processes and departments).
//! Uses polygram_core::ObjectGeom for object geometry with request/result containers.
//!
//! Декомпозирован по модулям (один файл — одна ответственность):
//! - `camera`      — камера холста (смещение, зум)
//! - `object_rect` — прямоугольник объекта в координатах сетки
//! - `hit_test`    — определение объекта/шапки/отдела под экранной позицией
//! - `icons`       — иконки Lucide
//! - `paint`       — рендер объектов и линий связи
//! - `paint_department` — рендер frames отделов
//! - `theme`       — тема оформления
//! - `modal`       — универсальная оболочка модальных окон
//! - `routing`     — маршрутизация линий связи

mod camera;
mod hit_test;
mod icons;
mod object_rect;
pub mod paint;
mod paint_department;
pub mod theme;
pub mod modal;
pub mod routing;

pub use camera::Camera;
pub use icons::{paint_lucide, default_icon_for, lucide_by_name};
pub use object_rect::ObjectRect;
pub use paint::{paint_add_row, paint_process, ProcessPaintResult};
pub use paint_department::paint_department_frame;
pub use theme::{status_dot_color, status_label_ru, Theme};
pub use polygram_core::types::{Direction, NodeKind, StatusCode};
pub use polygram_core::geometry::{EdgeGeom, EntryGeom, ObjectGeom};

use std::collections::HashSet;

/// Встроенные данные шрифта Inter Regular (400).
pub const INTER_REGULAR: &'static [u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");
/// Встроенные данные шрифта Inter Medium (500).
pub const INTER_MEDIUM: &'static [u8] = include_bytes!("../assets/fonts/Inter-Medium.ttf");
/// Встроенные данные шрифта Inter SemiBold (600).
pub const INTER_SEMIBOLD: &'static [u8] = include_bytes!("../assets/fonts/Inter-SemiBold.ttf");
/// Встроенные данные шрифта Inter Bold (700).
pub const INTER_BOLD: &'static [u8] = include_bytes!("../assets/fonts/Inter-Bold.ttf");

/// Регистрировать шрифты Inter (500/600/700) в контексте egui.
/// Вызывать один раз при инициализации приложения.
pub fn register_inter_fonts(ctx: &egui::Context) {
    use std::sync::Arc;
    let mut font_defs = egui::FontDefinitions::default();
    font_defs.font_data.insert(
        "inter_regular".to_owned(),
        Arc::new(egui::FontData::from_static(INTER_REGULAR)),
    );
    font_defs.font_data.insert(
        "inter_medium".to_owned(),
        Arc::new(egui::FontData::from_static(INTER_MEDIUM)),
    );
    font_defs.font_data.insert(
        "inter_semibold".to_owned(),
        Arc::new(egui::FontData::from_static(INTER_SEMIBOLD)),
    );
    font_defs.font_data.insert(
        "inter_bold".to_owned(),
        Arc::new(egui::FontData::from_static(INTER_BOLD)),
    );
    // Inter-Regular как первый выбор для пропорционального шрифта (fallback цепочка сохранит системные шрифты).
    if let Some(proportional) = font_defs.families.get_mut(&egui::FontFamily::Proportional) {
        proportional.insert(0, "inter_regular".to_owned());
    }
    ctx.set_fonts(font_defs);
}

use egui::{Context, Color32, PointerButton, Pos2, Rect, Response, Vec2};
use polygram_core::{DynamicState, id_allocator::allocate_id};

/// Canvas Polygram.
pub struct Canvas {
    /// Theme of the canvas (colors and geometry).
    pub theme: Theme,
    /// Camera of the canvas (offset and zoom).
    pub camera: Camera,
    /// Size of one grid cell in world coordinates.
    pub cell_size: f32,
    /// Objects on the canvas (processes and departments).
    pub objects: Vec<ObjectGeom>,
    /// Связи между показателями (ключи записей). Линии строятся из них и положения карточек.
    pub edges: Vec<EdgeGeom>,
    /// Кэш маршрутов линий связи (не сериализуется).
    edge_layer: routing::EdgeLayer,
    /// Горячие зоны добавления показателя (id объекта, направление, экранный rect).
    pub add_metric_hotspots: Vec<(String, Direction, Rect)>,
    /// Горячие зоны кнопки меню контейнера (id объекта, направление, экранный rect кнопки ≡).
    pub container_menu_hotspots: Vec<(String, Direction, Rect)>,
    /// Горячие зоны заголовка контейнера (id объекта, направление, экранный rect шапки).
    pub container_header_hotspots: Vec<(String, Direction, Rect)>,
    /// Горячие зоны строк показателей для перетаскивания (id объекта, направление, имя показателя, экранный rect строки).
    pub drag_metric_hotspots: Vec<(String, Direction, String, Rect)>,
}

impl Default for Canvas {
    fn default() -> Self {
        Self::new()
    }
}

impl Canvas {
    /// Creates a new canvas with the dark theme, default camera, and 16.0 cell size.
    pub fn new() -> Self {
        Self {
            theme: Theme::dark(),
            camera: Camera::new(),
            cell_size: 16.0,
            objects: Vec::new(),
            edges: Vec::new(),
            edge_layer: routing::EdgeLayer::new(),
            add_metric_hotspots: Vec::new(),
            container_menu_hotspots: Vec::new(),
            container_header_hotspots: Vec::new(),
            drag_metric_hotspots: Vec::new(),
        }
    }

    /// Returns the size of one cell in pixels, accounting for zoom.
    pub fn cell_px(&self) -> f32 {
        self.cell_size * self.camera.zoom
    }

    /// Converts a screen position to grid cell coordinates, snapped to integers.
    pub fn screen_to_cell(&self, screen_pos: Pos2) -> (i32, i32) {
        let zoom = self.camera.zoom;
        let offset = self.camera.offset;
        let cx = ((screen_pos.x / zoom - offset.x) / self.cell_size).round() as i32;
        let cy = ((screen_pos.y / zoom - offset.y) / self.cell_size).round() as i32;
        (cx, cy)
    }

    /// Converts a world position (grid cells) to screen pixels.
    pub fn cell_to_screen(&self, cell_x: i32, cell_y: i32) -> Pos2 {
        let zoom = self.camera.zoom;
        let offset = self.camera.offset;
        Pos2::new(
            (cell_x as f32 * self.cell_size + offset.x) * zoom,
            (cell_y as f32 * self.cell_size + offset.y) * zoom,
        )
    }

    /// Обработка камеры холста: зум колёсиком мыши к курсору и
    /// панорамирование правой или средней кнопкой мыши.
    pub fn handle_camera(
        &mut self,
        ctx: &Context,
        canvas_response: &Response,
        ui: &egui::Ui,
    ) {
        if !canvas_response.hovered() {
            return;
        }

        let scroll = ctx.input(|i| i.smooth_scroll_delta());
        if scroll.y.abs() > 0.001 {
            if let Some(cursor) = ctx.pointer_hover_pos() {
                let old_zoom = self.camera.zoom;
                let world_cursor = Pos2::new(
                    (cursor.x / old_zoom - self.camera.offset.x) / self.cell_size,
                    (cursor.y / old_zoom - self.camera.offset.y) / self.cell_size,
                );
                let factor = (scroll.y * 0.01).exp();
                let new_zoom = (old_zoom * factor).clamp(0.1, 10.0);
                if new_zoom != old_zoom {
                    self.camera.offset.x = cursor.x / new_zoom - world_cursor.x * self.cell_size;
                    self.camera.offset.y = cursor.y / new_zoom - world_cursor.y * self.cell_size;
                    self.camera.zoom = new_zoom;
                    ctx.request_repaint();
                }
            }
        }

        // Панорамирование камеры свободно (без привязки к сетке во время перетаскивания).
        if canvas_response.dragged_by(PointerButton::Secondary)
            || canvas_response.dragged_by(PointerButton::Middle)
        {
            let delta = ui.input(|i| i.pointer.delta());
            if delta.x != 0.0 || delta.y != 0.0 {
                self.camera.offset += delta;
                ctx.request_repaint();
            }
        }

        // При отпускании кнопки панорамирования камера стыкуется к ближайшим
        // координатам выравнивания сетки.
        if canvas_response.drag_stopped_by(PointerButton::Secondary)
            || canvas_response.drag_stopped_by(PointerButton::Middle)
        {
            self.camera.offset.x = (self.camera.offset.x / self.cell_size).round() * self.cell_size;
            self.camera.offset.y = (self.camera.offset.y / self.cell_size).round() * self.cell_size;
            ctx.request_repaint();
        }
    }

    /// Adds a department at the given grid cell position.
    pub fn add_department(&mut self, cell_x: i32, cell_y: i32, name: impl Into<String>) -> String {
        let existing_ids: Vec<String> = self.objects.iter().map(|o| o.id.clone()).collect();
        let id = allocate_id(&existing_ids);
        let name = name.into();
        let ix = self.objects.len() as u32;
            self.objects.push(ObjectGeom {
                ix,
                id: id.clone(),
                name,
                kind: NodeKind::Department,
                rect: polygram_core::layout::Rect::new(cell_x, cell_y, 30, 30),
                direction: None,
                requests: Vec::new(),
                results: Vec::new(),
                parent: None,
                icon: String::new(),
            });
        id
    }

    /// Adds a process at the given grid cell position.
    pub fn add_process(&mut self, cell_x: i32, cell_y: i32, name: impl Into<String>) -> String {
        let existing_ids: Vec<String> = self.objects.iter().map(|o| o.id.clone()).collect();
        let id = allocate_id(&existing_ids);
        let name = name.into();
        let ix = self.objects.len() as u32;
            self.objects.push(ObjectGeom {
                ix,
                id: id.clone(),
                name,
                kind: NodeKind::Process,
                rect: polygram_core::layout::Rect::new(cell_x, cell_y, 16, 16),
                direction: None,
                requests: Vec::new(),
                results: Vec::new(),
                parent: None,
                icon: String::new(),
            });
        id
    }

    /// Main render function: background + dot grid + department frames + process cards.
    pub fn paint(
        &mut self,
        ui: &mut egui::Ui,
        dynamic: &DynamicState,
        hovered_object: Option<&str>,
        selected_objects: &[String],
        drag_visual: Option<(egui::Vec2, &HashSet<String>)>,
    ) {
        let painter = ui.painter();
        let clip_rect = ui.clip_rect();
        let cell_px = self.cell_px();
        let zoom = self.camera.zoom;
        let offset = self.camera.offset;
        let grid_color = self.theme.grid_color;
        let dot_radius_world = self.theme.dot_radius_world;

        let pointer_pos = ui.ctx().input(|i| i.pointer.hover_pos());

        self.add_metric_hotspots.clear();
        self.container_menu_hotspots.clear();
        self.container_header_hotspots.clear();
        self.drag_metric_hotspots.clear();

        // Canvas background.
        painter.rect_filled(clip_rect, 0.0, self.theme.bg);

        // Dot grid with fade on zoom out.
        const FADE_ZOOM_MAX: f32 = 1.0;
        const FADE_ZOOM_MIN: f32 = 0.2;
        let alpha = ((zoom - FADE_ZOOM_MIN) / (FADE_ZOOM_MAX - FADE_ZOOM_MIN)).clamp(0.0, 1.0);
        if alpha > 0.0 {
            let color = Color32::from_rgba_unmultiplied(
                grid_color.r(),
                grid_color.g(),
                grid_color.b(),
                (255.0 * alpha) as u8,
            );
            let dot_radius_px = (dot_radius_world * cell_px).max(0.5);

            let min_cx = ((clip_rect.min.x / zoom - offset.x) / self.cell_size).floor() as i32;
            let max_cx = ((clip_rect.max.x / zoom - offset.x) / self.cell_size).ceil() as i32;
            let min_cy = ((clip_rect.min.y / zoom - offset.y) / self.cell_size).floor() as i32;
            let max_cy = ((clip_rect.max.y / zoom - offset.y) / self.cell_size).ceil() as i32;

            for c in min_cx..=max_cx {
                let sx = (c as f32 * self.cell_size + offset.x) * zoom;
                for r in min_cy..=max_cy {
                    let sy = (r as f32 * self.cell_size + offset.y) * zoom;
                    painter.circle_filled(Pos2::new(sx, sy), dot_radius_px, color);
                }
            }
        }

        // Department frames: hovered departments first (normal header/content, drawn behind children).
        for object in &self.objects {
            if object.kind != NodeKind::Department {
                continue;
            }
            let r = &object.rect;
            let mut screen_pos = self.cell_to_screen(r.x, r.y);
            if let Some((shift, ids)) = &drag_visual {
                if ids.contains(&object.id) {
                    screen_pos = screen_pos + *shift;
                }
            }
            let rect_px = Rect::from_min_size(
                screen_pos,
                Vec2::new(r.w as f32 * cell_px, r.h as f32 * cell_px),
            );
            if !rect_px.intersects(clip_rect) {
                continue;
            }
            let is_selected = selected_objects.iter().any(|id| id == &object.id);
            let is_hovered = pointer_pos.map_or(false, |p| rect_px.contains(p));
            if !is_hovered {
                continue; // Nicht-hovered отделы рисуются обложкой в третьем проходе (поверх детей).
            }
            // Отдел, прикрытый обложкой не наведённого родительского отдела,
            // не рисуется своим содержимым — родительская обложка (третий проход)
            // перекрывает всё его содержимое, как у дочерних процессов.
            if self.department_covered_by_cover(pointer_pos, object) {
                continue;
            }
            let process_count = self.objects.iter()
                .filter(|o| o.kind == NodeKind::Process && o.parent.as_ref() == Some(&object.id))
                .count();
            paint_department_frame(
                painter,
                ui,
                object,
                rect_px,
                zoom,
                &self.theme,
                is_hovered,
                process_count,
            );
        }

        // Process cards with containers.
        // Вычислить наложения процессов: для каждой пары пересекающихся процессов
        // самая верхняя — объект с бóльшим индексом в self.objects (z-порядок).
        let process_indices: Vec<usize> = self
            .objects
            .iter()
            .enumerate()
            .filter(|(_, o)| o.kind == NodeKind::Process)
            .map(|(i, _)| i)
            .collect();
        let mut is_top_overlap: Vec<bool> = vec![false; self.objects.len()];
        for ai in 0..process_indices.len() {
            for bi in (ai + 1)..process_indices.len() {
                let ia = process_indices[ai];
                let ib = process_indices[bi];
                let oa = &self.objects[ia];
                let ob = &self.objects[ib];
                let mut pos_a = self.cell_to_screen(oa.rect.x, oa.rect.y);
                let mut pos_b = self.cell_to_screen(ob.rect.x, ob.rect.y);
                if let Some((shift, ids)) = &drag_visual {
                    if ids.contains(&oa.id) {
                        pos_a = pos_a + *shift;
                    }
                    if ids.contains(&ob.id) {
                        pos_b = pos_b + *shift;
                    }
                }
                let card_w = 512.0 * zoom;
                let rect_a = Rect::from_min_size(
                    pos_a,
                    Vec2::new(card_w, crate::paint::object::process_total_height(oa, dynamic, zoom, card_w)),
                );
                let rect_b = Rect::from_min_size(
                    pos_b,
                    Vec2::new(card_w, crate::paint::object::process_total_height(ob, dynamic, zoom, card_w)),
                );
                if rect_a.intersects(rect_b) {
                    let top = if ia > ib { ia } else { ib };
                    is_top_overlap[top] = true;
                }
            }
        }

        for (ix, object) in self.objects.iter().enumerate() {
            if object.kind != NodeKind::Process {
                continue;
            }
            // Если процесс находится внутри отдела, который показывает обложку
            // (не наведён), не рисуем процесс — обложка отдела в третьем проходе
            // перекрывает всё его содержимое. Проверяем всю цепочку родителей.
            let mut skip_process = false;
            let mut parent_id = object.parent.clone();
            while let Some(pid) = parent_id {
                if let Some(parent_obj) = self.objects.iter().find(|o| o.id == pid) {
                    if parent_obj.kind == NodeKind::Department {
                        let pr = &parent_obj.rect;
                        let mut ppos = self.cell_to_screen(pr.x, pr.y);
                        if let Some((shift, ids)) = &drag_visual {
                            if ids.contains(&parent_obj.id) {
                                ppos = ppos + *shift;
                            }
                        }
                        let prect = Rect::from_min_size(
                            ppos,
                            Vec2::new(pr.w as f32 * cell_px, pr.h as f32 * cell_px),
                        );
                        let p_hovered = pointer_pos.map_or(false, |p| prect.contains(p));
                        if !p_hovered {
                            skip_process = true;
                            break;
                        }
                        parent_id = parent_obj.parent.clone();
                    } else {
                        parent_id = parent_obj.parent.clone();
                    }
                } else {
                    break;
                }
            }
            if skip_process {
                continue;
            }

            let r = &object.rect;
            let mut screen_pos = self.cell_to_screen(r.x, r.y);
            if let Some((shift, ids)) = &drag_visual {
                if ids.contains(&object.id) {
                    screen_pos = screen_pos + *shift;
                }
            }
            // Процесс: ширина фиксирована 512px (CSS спецификация), высота динамическая.
            let card_w = 512.0 * zoom;
            let card_h = crate::paint::object::process_total_height(object, dynamic, zoom, card_w);
            let rect_px = Rect::from_min_size(screen_pos, Vec2::new(card_w, card_h));
            if !rect_px.intersects(clip_rect) {
                continue;
            }
            let is_selected = selected_objects.iter().any(|id| id == &object.id);
            let mouse_pos = pointer_pos;
            let result = paint_process(
                ui,
                painter,
                object,
                dynamic,
                &self.theme,
                rect_px.min.x,
                rect_px.min.y,
                card_w,
                zoom,
                mouse_pos,
                is_selected,
                is_top_overlap[ix],
            );
            self.add_metric_hotspots.extend(result.add_hotspots);
            self.container_menu_hotspots.extend(result.menu_hotspots);
            self.drag_metric_hotspots.extend(result.drag_hotspots);
            self.container_header_hotspots.extend(result.header_hotspots);
        }

        // Department covers for non-hovered departments — рисуются поверх дочерних процессов.
        for object in &self.objects {
            if object.kind != NodeKind::Department {
                continue;
            }
            let r = &object.rect;
            let mut screen_pos = self.cell_to_screen(r.x, r.y);
            if let Some((shift, ids)) = &drag_visual {
                if ids.contains(&object.id) {
                    screen_pos = screen_pos + *shift;
                }
            }
            let rect_px = Rect::from_min_size(
                screen_pos,
                Vec2::new(r.w as f32 * cell_px, r.h as f32 * cell_px),
            );
            if !rect_px.intersects(clip_rect) {
                continue;
            }
            let is_hovered = pointer_pos.map_or(false, |p| rect_px.contains(p));
            if is_hovered {
                continue; // Наведённый отдел уже нарисован в первом проходе (обычное содержимое).
            }
            // Отдел, прикрытый обложкой не наведённого родительского отдела,
            // не рисуется своей обложкой — родительская обложка уже перекрывает его.
            if self.department_covered_by_cover(pointer_pos, object) {
                continue;
            }
            let process_count = self.objects.iter()
                .filter(|o| o.kind == NodeKind::Process && o.parent.as_ref() == Some(&object.id))
                .count();
            paint_department_frame(
                painter,
                ui,
                object,
                rect_px,
                zoom,
                &self.theme,
                is_hovered,
                process_count,
            );
        }

        // Линии связи рисуются поверх карточек и обложек.
        paint::edge::paint_edges(
            painter,
            clip_rect,
            &self.objects,
            &self.edges,
            &mut self.edge_layer,
            dynamic,
            &self.theme,
            &self.camera,
            self.cell_size,
            drag_visual,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use polygram_core::layout::Rect;
    use polygram_core::types::NodeKind;

    fn dept(id: &str, rect: Rect, parent: Option<&str>) -> ObjectGeom {
        ObjectGeom {
            ix: 0,
            id: id.to_owned(),
            name: id.to_owned(),
            kind: NodeKind::Department,
            rect,
            direction: None,
            requests: Vec::new(),
            results: Vec::new(),
            parent: parent.map(|p| p.to_owned()),
            icon: String::new(),
        }
    }

    fn proc(id: &str, rect: Rect) -> ObjectGeom {
        ObjectGeom {
            ix: 0,
            id: id.to_owned(),
            name: id.to_owned(),
            kind: NodeKind::Process,
            rect,
            direction: None,
            requests: Vec::new(),
            results: Vec::new(),
            parent: None,
            icon: String::new(),
        }
    }

    /// Поставить ix равным индексу в векторе (z-порядок).
    fn fix_ixs(objects: &mut Vec<ObjectGeom>) {
        for (i, o) in objects.iter_mut().enumerate() {
            o.ix = i as u32;
        }
    }

    /// Экранный rect карточки процесса (как в Canvas::paint: cell_size=16, zoom=1, card_w=512).
    fn screen_rect(o: &ObjectGeom) -> egui::Rect {
        let pos = egui::pos2(o.rect.x as f32 * 16.0, o.rect.y as f32 * 16.0);
        egui::Rect::from_min_size(pos, egui::Vec2::new(512.0, 300.0))
    }

    /// Реплика логики определения top-overlap из Canvas::paint для тестирования.
    fn top_overlap_for(objects: &[ObjectGeom]) -> Vec<bool> {
        let mut is_top = vec![false; objects.len()];
        let proc_idxs: Vec<usize> = objects
            .iter()
            .enumerate()
            .filter(|(_, o)| o.kind == NodeKind::Process)
            .map(|(i, _)| i)
            .collect();
        for ai in 0..proc_idxs.len() {
            for bi in (ai + 1)..proc_idxs.len() {
                let ia = proc_idxs[ai];
                let ib = proc_idxs[bi];
                if screen_rect(&objects[ia]).intersects(screen_rect(&objects[ib])) {
                    let top = if ia > ib { ia } else { ib };
                    is_top[top] = true;
                }
            }
        }
        is_top
    }

    #[test]
    fn child_department_covered_by_non_hovered_parent() {
        let mut canvas = Canvas::new();
        canvas.objects.push(dept("d1", Rect::new(0, 0, 30, 30), None));
        canvas.objects.push(dept("d2", Rect::new(5, 5, 16, 16), Some("d1")));
        let d2 = &canvas.objects[1];

        // Без наведения оба отдела не наведены: дочерний d2 прикрыт обложкой родителя d1.
        assert!(canvas.department_covered_by_cover(None, d2));

        // Наведён родитель d1 — обложка родителя не показывается, дочерний d2 не прикрыт.
        let pos = canvas.cell_to_screen(15, 15); // внутри rect d1
        assert!(!canvas.department_covered_by_cover(Some(pos), d2));
    }

    #[test]
    fn top_level_department_not_covered() {
        let mut canvas = Canvas::new();
        canvas.objects.push(dept("d1", Rect::new(0, 0, 30, 30), None));
        let d1 = &canvas.objects[0];
        assert!(!canvas.department_covered_by_cover(None, d1));
    }

    #[test]
    fn overlapping_processes_higher_index_is_top() {
        let mut canvas = Canvas::new();
        canvas.objects.push(proc("p1", Rect::new(0, 0, 16, 16)));
        canvas.objects.push(proc("p2", Rect::new(10, 10, 16, 16))); // карты наслаиваются (512px ширина)
        fix_ixs(&mut canvas.objects);
        let top = top_overlap_for(&canvas.objects);
        assert!(!top[0]);
        assert!(top[1]);
    }

    #[test]
    fn non_overlapping_processes_no_top_overlap() {
        let mut canvas = Canvas::new();
        canvas.objects.push(proc("p1", Rect::new(0, 0, 16, 16)));
        canvas.objects.push(proc("p2", Rect::new(100, 100, 16, 16))); // далеко, нет наложения
        fix_ixs(&mut canvas.objects);
        let top = top_overlap_for(&canvas.objects);
        assert!(!top[0]);
        assert!(!top[1]);
    }

    #[test]
    fn chain_overlaps_each_pair_top_is_higher_index() {
        let mut canvas = Canvas::new();
        canvas.objects.push(proc("p1", Rect::new(0, 0, 16, 16)));
        canvas.objects.push(proc("p2", Rect::new(10, 10, 16, 16))); // наслаивается на p1
        canvas.objects.push(proc("p3", Rect::new(20, 20, 16, 16))); // наслаивается на p2, не на p1
        fix_ixs(&mut canvas.objects);
        let top = top_overlap_for(&canvas.objects);
        assert!(!top[0]); // p1 проигрывает p2
        assert!(top[1]);  // p2 выигрывает у p1
        assert!(top[2]);  // p3 выигрывает у p2
    }
}

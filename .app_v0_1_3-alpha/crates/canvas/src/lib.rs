//! Canvas Polygram: background, dot grid, and objects (processes and departments).
//! Uses polygram_core::ObjectGeom for object geometry with request/result containers.

mod icons;
pub mod paint;
mod paint_department;
pub mod theme;
pub mod modal;

pub use icons::paint_lucide;
pub use paint::{paint_add_row, paint_process, ProcessPaintResult};
pub use paint_department::paint_department_frame;
pub use theme::{status_dot_color, status_label_ru, Theme};
pub use polygram_core::types::{Direction, NodeKind, StatusCode};
pub use polygram_core::geometry::{EntryGeom, ObjectGeom};

/// Встроенные данные шрифта Inter Regular (400).
pub const INTER_REGULAR: &'static [u8] = include_bytes!("../assets/fonts/Inter-Regular.otf");
/// Встроенные данные шрифта Inter Medium (500).
pub const INTER_MEDIUM: &'static [u8] = include_bytes!("../assets/fonts/Inter-Medium.otf");
/// Встроенные данные шрифта Inter SemiBold (600).
pub const INTER_SEMIBOLD: &'static [u8] = include_bytes!("../assets/fonts/Inter-SemiBold.otf");
/// Встроенные данные шрифта Inter Bold (700).
pub const INTER_BOLD: &'static [u8] = include_bytes!("../assets/fonts/Inter-Bold.otf");

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

/// Camera of the canvas: offset and zoom.
#[derive(Clone, Copy, Debug, Default)]
pub struct Camera {
    /// Camera offset in world coordinates.
    pub offset: egui::Vec2,
    /// Zoom factor.
    pub zoom: f32,
}

impl Camera {
    /// Creates a new camera with zero offset and zoom 1.0.
    pub fn new() -> Self {
        Self {
            offset: egui::Vec2::ZERO,
            zoom: 1.0,
        }
    }
}

/// Position and size of an object in world coordinates (grid cells).
#[derive(Clone, Debug, PartialEq)]
pub struct ObjectRect {
    /// Cell X coordinate of the top-left corner.
    pub x: i32,
    /// Cell Y coordinate of the top-left corner.
    pub y: i32,
    /// Width in cells.
    pub w: i32,
    /// Height in cells.
    pub h: i32,
}

impl ObjectRect {
    /// Creates a new object rectangle at the given cell position and size.
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }
}

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

        if canvas_response.dragged_by(PointerButton::Secondary)
            || canvas_response.dragged_by(PointerButton::Middle)
        {
            self.camera.offset += ui.input(|i| i.pointer.delta());
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

        // Department frames (container borders with header, drawn behind child cards).
        for object in &self.objects {
            if object.kind != NodeKind::Department {
                continue;
            }
            let r = &object.rect;
            let rect_px = Rect::from_min_size(
                self.cell_to_screen(r.x, r.y),
                Vec2::new(r.w as f32 * cell_px, r.h as f32 * cell_px),
            );
            if !rect_px.intersects(clip_rect) {
                continue;
            }
            let is_selected = selected_objects.iter().any(|id| id == &object.id);
            let is_highlighted = is_selected || pointer_pos.map_or(false, |p| rect_px.contains(p));
            // Подсчитать процессы внутри отдела.
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
                is_highlighted,
                process_count,
            );
        }

        // Process cards with containers.
        for object in &self.objects {
            if object.kind != NodeKind::Process {
                continue;
            }
            let r = &object.rect;
            let screen_pos = self.cell_to_screen(r.x, r.y);
            // Процесс: ширина фиксирована 512px (CSS спецификация), высота динамическая.
            let card_w = 512.0 * zoom;
            let card_h = paint::object::process_total_height(object, dynamic, zoom, card_w);
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
            );
            self.add_metric_hotspots.extend(result.add_hotspots);
            self.container_menu_hotspots.extend(result.menu_hotspots);
            self.drag_metric_hotspots.extend(result.drag_hotspots);
            self.container_header_hotspots.extend(result.header_hotspots);
        }
    }

    /// Вернуть id любого объекта (процесса или отдела), под которым находится заданная экранная позиция.
    /// Сначала проверяются процессы (с фиксированной геометрией 256×256px), затем отделы
    /// (возвращается самый вложенный — с наименьшей площадью rect).
    pub fn object_at_any<'a>(&'a self, screen_pos: Pos2, dynamic: &DynamicState) -> Option<&'a str> {
        let cell_px = self.cell_px();
        let zoom = self.camera.zoom;

        // Сначала процессы (динамическая CSS-геометрия).
        for object in &self.objects {
            if object.kind != NodeKind::Process {
                continue;
            }
            let r = &object.rect;
            let card_w = 512.0 * zoom;
            let card_h = paint::object::process_total_height(object, dynamic, zoom, card_w);
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
            let rect_px = egui::Rect::from_min_size(
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
            let header_rect = egui::Rect::from_min_size(
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

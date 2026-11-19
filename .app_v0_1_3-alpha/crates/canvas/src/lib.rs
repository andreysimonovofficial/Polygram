//! Canvas Polygram: background, dot grid, and objects (processes and departments).
//! Minimal rendering set: background fill, dot grid with fade on zoom out,
//! and department frames (container borders with header).

mod theme;

pub use theme::{StatusCode, Theme};

use egui::{Context, Color32, PointerButton, Pos2, Rect, Response, Stroke, Vec2};

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

/// Kind of a canvas object.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ObjectKind {
    /// Process — a card with metrics and ports.
    #[default]
    Process,
    /// Department — container frame drawn behind child cards.
    Department,
}

impl ObjectKind {
    /// Russian label of the object kind.
    pub fn label_ru(self) -> &'static str {
        match self {
            Self::Process => "Процесс",
            Self::Department => "Отдел",
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

/// Geometry of a single canvas object.
#[derive(Clone, Debug, PartialEq)]
pub struct ObjectGeom {
    /// Sequential index.
    pub ix: u32,
    /// Unique identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Kind of object (Process or Department).
    pub kind: ObjectKind,
    /// Position and size in world coordinates (grid cells).
    pub rect: ObjectRect,
    /// Id of the parent department, if nested inside one.
    pub parent: Option<String>,
    /// Aggregated status of the department (used for background fill).
    pub status: StatusCode,
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
    /// панорамирование правой или средней кнопкой мыши (как в старом приложении).
    /// Объекты на холсте адаптивно реагируют на изменение камеры при отрисовке
    /// через [`Canvas::paint`], так как все координаты переводятся через
    /// `cell_to_screen` с учётом `camera.zoom` и `camera.offset`.
    pub fn handle_camera(
        &mut self,
        ctx: &Context,
        canvas_response: &Response,
        ui: &egui::Ui,
    ) {
        if !canvas_response.hovered() {
            return;
        }

        // Зум колёсиком мыши к курсору: точка под курсором остаётся на месте.
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
                    // Новое смещение, чтобы та же мировая точка осталась под курсором.
                    self.camera.offset.x = cursor.x / new_zoom - world_cursor.x * self.cell_size;
                    self.camera.offset.y = cursor.y / new_zoom - world_cursor.y * self.cell_size;
                    self.camera.zoom = new_zoom;
                    ctx.request_repaint();
                }
            }
        }

        // Панорамирование правой или средней кнопкой мыши.
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
    /// Default size matches `footprint_for(NodeKind::Department, 0, 0)`:
    /// width NODE_W=30 cells, height min 30 cells (header + metric rows).
    pub fn add_department(&mut self, cell_x: i32, cell_y: i32, name: impl Into<String>) -> String {
        let id = format!("dept_{}", self.objects.len());
        let name = name.into();
        let ix = self.objects.len() as u32;
        self.objects.push(ObjectGeom {
            ix,
            id: id.clone(),
            name,
            kind: ObjectKind::Department,
            rect: ObjectRect::new(cell_x, cell_y, 30, 30),
            parent: None,
            status: StatusCode::Ok,
        });
        id
    }

    /// Main render function: background + dot grid + department frames.
    pub fn paint(
        &mut self,
        ui: &mut egui::Ui,
        selected_objects: &[String],
    ) {
        let painter = ui.painter();
        let clip_rect = ui.clip_rect();
        let cell_px = self.cell_px();
        let zoom = self.camera.zoom;
        let offset = self.camera.offset;
        let grid_color = self.theme.grid_color;
        let dot_radius_world = self.theme.dot_radius_world;

        // Pointer position in screen coordinates (same space as cell_to_screen).
        let pointer_pos = ui.ctx().input(|i| i.pointer.hover_pos());

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
            if object.kind != ObjectKind::Department {
                continue;
            }
            let r = ObjectRect::new(object.rect.x, object.rect.y, object.rect.w, object.rect.h);
            let rect_px = Rect::from_min_size(
                self.cell_to_screen(r.x, r.y),
                Vec2::new(r.w as f32 * cell_px, r.h as f32 * cell_px),
            );
            if !rect_px.intersects(clip_rect) {
                continue;
            }
            let is_selected = selected_objects.iter().any(|id| id == &object.id);
            let is_highlighted = is_selected || pointer_pos.map_or(false, |p| rect_px.contains(p));
            paint_department_frame(
                painter,
                object,
                rect_px,
                cell_px,
                zoom,
                &self.theme,
                object.status,
                is_highlighted,
            );
        }
    }
}

/// Draw a department frame: background fill, outer border, and header with title.
pub fn paint_department_frame(
    painter: &egui::Painter,
    object: &ObjectGeom,
    rect_px: Rect,
    cell_px: f32,
    zoom: f32,
    theme: &Theme,
    status: StatusCode,
    is_highlighted: bool,
) {
    // Background fill of the department — color by aggregated status.
    painter.rect_filled(
        rect_px,
        theme.object_corner_radius * zoom,
        theme.status_fill_color(status),
    );

    // Outer border — thin white on hover/select, normal color otherwise (as in old app).
    let border_color = if is_highlighted {
        theme.text
    } else {
        theme.object_border
    };
    let border_width = if is_highlighted {
        theme.object_border_hover_width * zoom
    } else {
        theme.object_border_width * zoom
    };
    painter.rect_stroke(
        rect_px,
        theme.object_corner_radius * zoom,
        Stroke::new(border_width, border_color),
        egui::StrokeKind::Middle,
    );

    // Department header — title text only, no background.
    let header_h = cell_px * theme.object_header_height_cells;
    let title = object.name.clone();
    painter.text(
        rect_px.min + Vec2::new(12.0 * zoom, header_h / 2.0),
        egui::Align2::LEFT_CENTER,
        &title,
        egui::FontId::new(theme.text_size * zoom, egui::FontFamily::Proportional),
        theme.text,
    );

    let id_text = format!("#{}", object.ix);
    painter.text(
        Pos2::new(rect_px.max.x - 8.0 * zoom, rect_px.min.y + header_h / 2.0),
        egui::Align2::RIGHT_CENTER,
        &id_text,
        egui::FontId::new(theme.text_size * zoom, egui::FontFamily::Proportional),
        theme.text_muted,
    );
}

/// Вернуть id отдела, под которым находится заданная экранная позиция.
pub fn object_at<'a>(objects: &'a [ObjectGeom], screen_pos: Pos2, cell_px: f32) -> Option<&'a str> {
    for object in objects {
        if object.kind != ObjectKind::Department {
            continue;
        }
        let r = &object.rect;
        let rect_px = egui::Rect::from_min_size(
            egui::pos2(
                r.x as f32 * cell_px,
                r.y as f32 * cell_px,
            ),
            egui::Vec2::new(r.w as f32 * cell_px, r.h as f32 * cell_px),
        );
        // NOTE: caller must pass screen_pos already converted to canvas-local space.
        if rect_px.contains(screen_pos) {
            return Some(&object.id);
        }
    }
    None
}

/// Вернуть id отдела, чья шапка содержит заданную экранную позицию.
pub fn header_at<'a>(
    objects: &'a [ObjectGeom],
    screen_pos: Pos2,
    cell_px: f32,
) -> Option<&'a str> {
    for object in objects {
        if object.kind != ObjectKind::Department {
            continue;
        }
        let r = &object.rect;
        let header_h = cell_px * 1.0;
        let header_rect = egui::Rect::from_min_size(
            egui::pos2(
                r.x as f32 * cell_px,
                r.y as f32 * cell_px,
            ),
            egui::Vec2::new(r.w as f32 * cell_px, header_h),
        );
        if header_rect.contains(screen_pos) {
            return Some(&object.id);
        }
    }
    None
}

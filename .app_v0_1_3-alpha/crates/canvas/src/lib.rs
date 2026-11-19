//! Canvas Polygram: background and dot grid.
//! Minimal rendering set: background fill and dot grid with fade on zoom out.

mod theme;

pub use theme::Theme;

use egui::{Color32, Pos2};

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

/// Canvas Polygram.
pub struct Canvas {
    /// Theme of the canvas (colors and geometry).
    pub theme: Theme,
    /// Camera of the canvas (offset and zoom).
    pub camera: Camera,
    /// Size of one grid cell in world coordinates.
    pub cell_size: f32,
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
        }
    }

    /// Returns the size of one cell in pixels, accounting for zoom.
    pub fn cell_px(&self) -> f32 {
        self.cell_size * self.camera.zoom
    }

    /// Main render function: background + dot grid (no lines, no objects).
    pub fn paint(&mut self, ui: &mut egui::Ui) {
        let painter = ui.painter();
        let clip_rect = ui.clip_rect();
        let cell_px = self.cell_px();
        let zoom = self.camera.zoom;
        let offset = self.camera.offset;
        let grid_color = self.theme.grid_color;
        let dot_radius_world = self.theme.dot_radius_world;

        // Canvas background.
        painter.rect_filled(clip_rect, 0.0, self.theme.bg);

        // Dot grid with fade on zoom out.
        const FADE_ZOOM_MAX: f32 = 1.0;
        const FADE_ZOOM_MIN: f32 = 0.2;
        let alpha = ((zoom - FADE_ZOOM_MIN) / (FADE_ZOOM_MAX - FADE_ZOOM_MIN)).clamp(0.0, 1.0);
        if alpha <= 0.0 {
            return;
        }

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
}

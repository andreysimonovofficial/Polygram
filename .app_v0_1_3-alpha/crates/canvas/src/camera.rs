//! Камера холста: смещение и зум.

use egui::Vec2;

/// Camera of the canvas: offset and zoom.
#[derive(Clone, Copy, Debug, Default)]
pub struct Camera {
    /// Camera offset in world coordinates.
    pub offset: Vec2,
    /// Zoom factor.
    pub zoom: f32,
}

impl Camera {
    /// Creates a new camera with zero offset and zoom 1.0.
    pub fn new() -> Self {
        Self {
            offset: Vec2::ZERO,
            zoom: 1.0,
        }
    }
}

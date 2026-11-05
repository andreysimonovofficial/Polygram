use egui::{Pos2, Vec2};

/// Состояние камеры.
#[derive(Clone, Debug, Default)]
pub struct Camera {
    pub offset: Vec2,
    pub zoom: f32,
}

impl Camera {
    pub fn new() -> Self {
        Self { offset: Vec2::ZERO, zoom: 1.0 }
    }

    pub fn world_to_screen(&self, p: Pos2, cell_size: f32) -> Pos2 {
        Pos2::new(
            (p.x * cell_size + self.offset.x) * self.zoom,
            (p.y * cell_size + self.offset.y) * self.zoom,
        )
    }

    pub fn screen_to_world(&self, p: Pos2, cell_size: f32) -> Pos2 {
        Pos2::new(
            (p.x / self.zoom - self.offset.x) / cell_size,
            (p.y / self.zoom - self.offset.y) / cell_size,
        )
    }
}

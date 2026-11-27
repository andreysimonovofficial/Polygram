//! Прямоугольник объекта в координатах сетки холста.

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

//! Модуль маршрутизации линий связи: привязка к сетке, проверка коллизий,
//! упрощение ломаной.

pub mod grid;
pub mod ports;

pub use grid::simplify_points;
pub use grid::{horizontal_blocked, obj_screen_rect, snap_grid_x_ceil, snap_grid_x_floor, vertical_blocked};
pub use ports::visual_port_screen;

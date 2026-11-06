//! Инструмент дизайна Polygram.
//! Позволяет визуально настраивать стили объектов холста (цвета, скругления, шрифты, рамки)
//! и рисовать свободные фигуры (прямоугольники, линии, текст, эллипсы) на Figma-подобном холсте.
//! Экспортирует результат в JSON, который Агент отрисовывает в точности.

mod design_app;
mod design_format;
mod exporter;
mod free_canvas;

pub use design_app::DesignApp;
pub use design_format::*;
pub use exporter::export_design;
pub use free_canvas::FreeCanvas;

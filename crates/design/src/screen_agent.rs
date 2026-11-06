//! Агент «Снимок экрана → фигуры» (пиксельная копия).
//! Пользователь прикрепляет файл скриншота, агент копирует его пиксель-в-пиксель:
//! каждая горизонтальная линия одинаковых пикселей становится одной фигурой Rect
//! с точным цветом. Изображение восстанавливается на холсте без потерь.

use crate::design_format::{DrawTool, FreeShape, Rgba};
use crate::free_canvas::FreeCanvas;

/// Агент пиксельной копии.
pub struct ScreenAgent;

/// Результат копирования.
pub struct CopyResult {
    pub shapes: usize,
    pub image_size: (u32, u32),
}

impl ScreenAgent {
    /// Загрузить изображение из файла (png, jpg, bmp и т. п.).
    pub fn load_image(path: &std::path::Path) -> Option<image::RgbaImage> {
        image::open(path).ok().map(|img| img.to_rgba8())
    }

    /// Превратить изображение в пиксельно-точные фигуры.
    /// Горизонтальные прогоны одинакового цвета сливаются в одну фигуру.
    pub fn image_to_shapes(img: &image::RgbaImage) -> Vec<FreeShape> {
        let (width, height) = img.dimensions();
        let pixels = img.as_raw();
        let stride = width as usize * 4;
        let mut shapes: Vec<FreeShape> = Vec::new();
        let mut next_id: u64 = 0;

        for y in 0..height {
            let row_start = (y as usize) * stride;
            let mut x = 0;
            while x < width {
                let offset = row_start + x as usize * 4;
                let r = pixels[offset];
                let g = pixels[offset + 1];
                let b = pixels[offset + 2];
                let a = pixels[offset + 3];

                let mut run: u32 = 1;
                while x + run < width {
                    let roffset = row_start + (x + run) as usize * 4;
                    if pixels[roffset] != r
                        || pixels[roffset + 1] != g
                        || pixels[roffset + 2] != b
                        || pixels[roffset + 3] != a
                    {
                        break;
                    }
                    run += 1;
                }

                let mut shape = FreeShape::default();
                shape.id = format!("px-{}", next_id);
                next_id += 1;
                shape.tool = DrawTool::Rect;
                shape.x = x as f32;
                shape.y = y as f32;
                shape.w = run as f32;
                shape.h = 1.0;
                shape.fill = Some(Rgba::from_rgba(r, g, b, a));
                shape.stroke = Rgba::from_rgb(0, 0, 0);
                shape.stroke_width = 0.0;
                shapes.push(shape);

                x += run;
            }
        }
        shapes
    }

    /// Применить изображение к холсту: очистить и нарисовать пиксель-в-пиксель.
    pub fn apply_image_to_canvas(canvas: &mut FreeCanvas, img: &image::RgbaImage) -> CopyResult {
        let shapes = Self::image_to_shapes(img);
        let count = shapes.len();
        let size = img.dimensions();
        canvas.save_undo_state();
        canvas.doc.shapes.clear();
        canvas.selected.clear();
        canvas.doc.shapes = shapes;
        canvas.touch();
        CopyResult { shapes: count, image_size: size }
    }
}

//! Агент «Снимок экрана → фигуры».
//! Делает снимок основного монитора через DXGI Desktop Duplication,
//! находит прямоугольные области на картинке и добавляет их на свободный холст
//! как фигуры FreeShape::Rect со sampled цветом.

use crate::design_format::{DrawTool, FreeShape, Rgba};
use crate::free_canvas::FreeCanvas;
use image::RgbaImage;

/// Агент: снимок экрана → фигуры.
pub struct ScreenAgent;

/// Настройки агента.
#[derive(Clone, Copy, Debug, Default)]
pub struct ScreenAgentConfig {
    /// Минимальная площадь области (в пикселях), чтобы стать фигурой.
    pub min_area: u32,
    /// Порог различия цвета от фона (0..=255), выше — только резкие границы.
    pub color_tolerance: u8,
}

/// Прямоугольная область: (лево, верх, ширина, высота).
type ScreenRect = (i32, i32, u32, u32);

impl ScreenAgent {
    /// Сделать снимок основного монитора.
    pub fn capture_screen() -> Option<RgbaImage> {
        lx_capture::DxgiCapture::new(lx_capture::CaptureColorDepth::TrueColor)
            .ok()
            .and_then(|mut cap| {
                cap.capture_initial().ok()?;
                let (width, height) = cap.dimensions();
                let pixels = cap.pixels(); // BGR24 packed
                let mut img = RgbaImage::new(width, height);
                for y in 0..height {
                    for x in 0..width {
                        let i = ((y as usize) * width as usize + x as usize) * 3;
                        let b = pixels[i];
                        let g = pixels[i + 1];
                        let r = pixels[i + 2];
                        img.put_pixel(x, y, image::Rgba([r, g, b, 255]));
                    }
                }
                Some(img)
            })
    }

    /// Найти прямоугольные области на изображении (компоненты связности).
    /// Возвращает ((лево, верх, ширина, высота), средний цвет) для каждой области.
    pub fn detect_rectangles(
        img: &RgbaImage,
        config: ScreenAgentConfig,
    ) -> Vec<(ScreenRect, Rgba)> {
        let (width, height) = img.dimensions();
        let gray = image::imageops::grayscale(img);
        let bg = Self::dominant_bg(&gray);

        let mut visited = vec![false; (width * height) as usize];
        let mut regions: Vec<(ScreenRect, Rgba)> = Vec::new();

        for y in 0..height {
            for x in 0..width {
                let idx = (y as usize) * width as usize + x as usize;
                if visited[idx] {
                    continue;
                }
                visited[idx] = true;
                let pixel = gray.get_pixel(x, y).0[0];
                if pixel.abs_diff(bg) < config.color_tolerance {
                    continue;
                }

                // BFS компонента связности (4-связность).
                let mut min_x = x;
                let mut min_y = y;
                let mut max_x = x;
                let mut max_y = y;
                let mut area: u32 = 0;
                let mut color_sum = [0u64; 4];

                let mut stack: Vec<(u32, u32)> = Vec::new();
                stack.push((x, y));
                while let Some((cx, cy)) = stack.pop() {
                    let cidx = (cy as usize) * width as usize + cx as usize;
                    if visited[cidx] {
                        continue;
                    }
                    visited[cidx] = true;
                    let cpixel = gray.get_pixel(cx, cy).0[0];
                    if cpixel.abs_diff(bg) < config.color_tolerance {
                        continue;
                    }
                    area += 1;
                    min_x = min_x.min(cx);
                    min_y = min_y.min(cy);
                    max_x = max_x.max(cx);
                    max_y = max_y.max(cy);
                    let rgba = img.get_pixel(cx, cy).0;
                    color_sum[0] += rgba[0] as u64;
                    color_sum[1] += rgba[1] as u64;
                    color_sum[2] += rgba[2] as u64;
                    color_sum[3] += rgba[3] as u64;

                    if cx > 0 {
                        stack.push((cx - 1, cy));
                    }
                    if cx + 1 < width {
                        stack.push((cx + 1, cy));
                    }
                    if cy > 0 {
                        stack.push((cx, cy - 1));
                    }
                    if cy + 1 < height {
                        stack.push((cx, cy + 1));
                    }
                }

                if area >= config.min_area && area > 0 {
                    let rect: ScreenRect = (
                        min_x as i32,
                        min_y as i32,
                        (max_x - min_x + 1) as u32,
                        (max_y - min_y + 1) as u32,
                    );
                    let avg = Rgba::from_rgba(
                        (color_sum[0] / area as u64) as u8,
                        (color_sum[1] / area as u64) as u8,
                        (color_sum[2] / area as u64) as u8,
                        (color_sum[3] / area as u64) as u8,
                    );
                    regions.push((rect, avg));
                }
            }
        }

        regions
    }

    /// Доминирующий оттенок фона по четырём углам изображения.
    fn dominant_bg(gray: &image::GrayImage) -> u8 {
        let (width, height) = gray.dimensions();
        if width == 0 || height == 0 {
            return 0;
        }
        let corners = [
            gray.get_pixel(0, 0).0[0],
            gray.get_pixel(width.saturating_sub(1), 0).0[0],
            gray.get_pixel(0, height.saturating_sub(1)).0[0],
            gray.get_pixel(width.saturating_sub(1), height.saturating_sub(1)).0[0],
        ];
        let sum: u32 = corners.iter().map(|&v| v as u32).sum();
        (sum / 4) as u8
    }

    /// Применить снимок к холсту: очистить фигуры и добавить области снимка.
    pub fn apply_to_canvas(canvas: &mut FreeCanvas, img: &RgbaImage, config: ScreenAgentConfig) {
        let regions = Self::detect_rectangles(img, config);
        canvas.save_undo_state();
        canvas.doc.shapes.clear();
        canvas.selected.clear();
        for (i, ((left, top, w, h), color)) in regions.into_iter().enumerate() {
            let mut shape = FreeShape::default();
            shape.id = format!("screen-shape-{}", i);
            shape.tool = DrawTool::Rect;
            shape.x = left as f32;
            shape.y = top as f32;
            shape.w = w as f32;
            shape.h = h as f32;
            shape.fill = Some(color);
            shape.stroke = Rgba::from_rgb(255, 255, 255);
            shape.stroke_width = 1.0;
            canvas.doc.shapes.push(shape);
        }
        canvas.touch();
    }
}

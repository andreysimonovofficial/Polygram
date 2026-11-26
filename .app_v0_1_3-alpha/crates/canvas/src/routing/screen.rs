//! Перевод маршрута в экранные координаты и подгонка концов под реальные порты.

use egui::Pos2;

use super::model::{Pt, U};

/// Точки маршрута (субклетки) → экранные пиксели.
///
/// Расчётные порты в кэше построены при масштабе 1; на текущем масштабе, а также при плавном
/// визуальном смещении перетаскиваемой карточки реальные порты отличаются на доли клетки. Поэтому
/// первая и последняя точки заменяются реальными экранными портами, а соседние точки выравниваются
/// по их высоте: первый и последний отрезки остаются строго горизонтальными.
pub fn route_to_screen(
    points: &[Pt],
    cell_size: f32,
    offset: (f32, f32),
    zoom: f32,
    src_port: Pos2,
    dst_port: Pos2,
) -> Vec<Pos2> {
    let mut pts: Vec<Pos2> = points
        .iter()
        .map(|p| {
            Pos2::new(
                (p.x as f32 / U as f32 * cell_size + offset.0) * zoom,
                (p.y as f32 / U as f32 * cell_size + offset.1) * zoom,
            )
        })
        .collect();
    let n = pts.len();
    if n < 2 {
        return pts;
    }
    pts[0] = src_port;
    pts[n - 1] = dst_port;
    if n == 2 {
        pts[1].y = pts[0].y;
    } else {
        pts[1].y = src_port.y;
        pts[n - 2].y = dst_port.y;
    }
    pts
}

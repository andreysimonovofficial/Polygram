//! Геометрия строк Процесса и точки сцепления.
//!
//! Все размеры берутся из тех же констант, что и отрисовка карточки (`paint::constants`), и
//! считаются в пикселях при масштабе 1. Порт стоит на внешнем краю строки показателя, на высоте
//! её центра: левый порт — на левом краю строки, правый — на правом.

use polygram_core::types::Direction;

use super::model::Side;
use crate::paint::constants::{
    CONTAINER_HEADER_H_PX, CONTAINER_PADDING_PX, GAP_PX, PROCESS_CARD_W_PX, PROCESS_HEADER_H_PX,
    PROCESS_PADDING_PX, ROW_H_PX,
};

/// Высота контейнера: шапка + строки + промежутки + отступы. Пустой контейнер содержит
/// строку «+ Добавить показатель».
fn container_height_px(n: usize) -> f32 {
    let (p, h, r, g) = (CONTAINER_PADDING_PX, CONTAINER_HEADER_H_PX, ROW_H_PX, GAP_PX);
    if n == 0 {
        p + h + r + p
    } else {
        p + h + n as f32 * r + (n - 1) as f32 * g + p
    }
}

/// Размер карточки Процесса в пикселях при масштабе 1: (ширина, высота).
pub fn card_size_px(n_req: usize, n_res: usize) -> (f32, f32) {
    let h = PROCESS_PADDING_PX
        + PROCESS_HEADER_H_PX
        + GAP_PX
        + container_height_px(n_req)
        + GAP_PX
        + container_height_px(n_res)
        + PROCESS_PADDING_PX;
    (PROCESS_CARD_W_PX, h)
}

/// Смещение порта строки от левого верхнего угла карточки (пиксели, масштаб 1).
/// `n_req` нужен, чтобы найти начало контейнера «Результат».
pub fn row_port_px(n_req: usize, dir: Direction, row: usize, side: Side) -> (f32, f32) {
    let inset = PROCESS_PADDING_PX + CONTAINER_PADDING_PX;
    let x = match side {
        Side::Left => inset,
        Side::Right => PROCESS_CARD_W_PX - inset,
    };
    let req_top = PROCESS_PADDING_PX + PROCESS_HEADER_H_PX + GAP_PX;
    let container_top = match dir {
        Direction::Request => req_top,
        Direction::Result => req_top + container_height_px(n_req) + GAP_PX,
    };
    let y = container_top
        + CONTAINER_PADDING_PX
        + CONTAINER_HEADER_H_PX
        + row as f32 * (ROW_H_PX + GAP_PX)
        + ROW_H_PX / 2.0;
    (x, y)
}

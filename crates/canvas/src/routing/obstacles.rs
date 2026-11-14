//! Препятствия маршрутизатора: карточки с охранной зоной, проверки отрезков, окно поиска.
//!
//! Правило одно: линия не заходит *внутрь* (в открытую область) препятствия. Ехать по его
//! границе можно — охранная зона как раз и отодвигает эту границу от самой карточки.

use super::model::{Params, U};
use polygram_core::layout::Rect;

/// Чья это карточка относительно проверяемой связи.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Own {
    Other,
    Src,
    Dst,
}

/// Карточка, раздутая на охранную зону. Границы — линии сетки (в клетках).
#[derive(Clone, Copy, Debug)]
pub struct Obs {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
    pub own: Own,
}

impl Obs {
    pub fn new(r: &Rect, clear: i32, own: Own) -> Self {
        Self {
            x0: r.x - clear,
            y0: r.y - clear,
            x1: r.x + r.w + clear,
            y1: r.y + r.h + clear,
            own,
        }
    }
}

/// Раздутые карточки, которые могут оказаться рядом со связью.
pub fn build(obstacles: &[Rect], src: &Rect, dst: &Rect, par: Params, reach: i32) -> Vec<Obs> {
    let min_x = src.x.min(dst.x) - reach;
    let max_x = (src.x + src.w).max(dst.x + dst.w) + reach;
    let min_y = src.y.min(dst.y) - reach;
    let max_y = (src.y + src.h).max(dst.y + dst.h) + reach;
    obstacles
        .iter()
        .filter(|r| r.x < max_x && r.x + r.w > min_x && r.y < max_y && r.y + r.h > min_y)
        .map(|r| {
            let own = if r == src {
                Own::Src
            } else if r == dst {
                Own::Dst
            } else {
                Own::Other
            };
            Obs::new(r, par.clear, own)
        })
        .collect()
}

/// Свободен ли вертикальный отрезок на линии сетки `x` (клетки) между `ya` и `yb` (субклетки).
pub fn v_free(obs: &[Obs], x: i32, ya: i32, yb: i32) -> bool {
    let (lo, hi) = (ya.min(yb), ya.max(yb));
    !obs.iter()
        .any(|o| o.x0 < x && x < o.x1 && o.y0 * U < hi && lo < o.y1 * U)
}

/// Свободен ли горизонтальный отрезок на высоте `y` (субклетки) между `xa` и `xb` (субклетки).
/// Карточки, помеченные в `skip`, не учитываются (свой край у порта).
pub fn h_free(obs: &[Obs], skip: &[Own], y: i32, xa: i32, xb: i32) -> bool {
    let (lo, hi) = (xa.min(xb), xa.max(xb));
    !obs.iter().any(|o| {
        !skip.contains(&o.own) && o.y0 * U < y && y < o.y1 * U && o.x0 * U < hi && lo < o.x1 * U
    })
}

/// Окно поиска: вершины сетки `x0..=x1`, `y0..=y1` (клетки).
#[derive(Clone, Copy, Debug)]
pub struct Win {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

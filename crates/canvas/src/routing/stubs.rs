//! Первый и последний отрезки: до какой вертикали сетки они могут дотянуться.
//!
//! Отрезок идёт строго горизонтально от порта наружу от карточки до вертикали сетки,
//! которая лежит минимум в `min_stub` клетках от края карточки.

use super::model::{Params, Side, Terminal, U};
use super::obstacles::{Obs, Own};

/// Максимальный вылет горизонтального отрезка от карточки (клеток).
pub const MAX_REACH: i32 = 150;

/// Допустимые вертикали сетки `lo..=hi` для отрезка из порта стороны `side` на высоте
/// `y` (субклетки). `own` — какая карточка своя: её охранную зону отрезок пересекает
/// законно. `clamp` — необязательное ограничение окном поиска.
pub fn columns(
    obs: &[Obs],
    own: Own,
    t: &Terminal,
    side: Side,
    y: i32,
    par: Params,
    clamp: Option<(i32, i32)>,
) -> Option<(i32, i32)> {
    let edge = t.edge_x(side);
    let blocking = |o: &&Obs| o.own != own && o.y0 * U < y && y < o.y1 * U;
    let (mut lo, mut hi) = match side {
        Side::Right => {
            let near = edge + par.min_stub;
            let mut far = near + MAX_REACH;
            for o in obs.iter().filter(blocking) {
                if o.x1 > edge {
                    far = far.min(o.x0);
                }
            }
            (near, far)
        }
        Side::Left => {
            let near = edge - par.min_stub;
            let mut far = near - MAX_REACH;
            for o in obs.iter().filter(blocking) {
                if o.x0 < edge {
                    far = far.max(o.x1);
                }
            }
            (far, near)
        }
    };
    if let Some((c0, c1)) = clamp {
        lo = lo.max(c0);
        hi = hi.min(c1);
    }
    (lo <= hi).then_some((lo, hi))
}

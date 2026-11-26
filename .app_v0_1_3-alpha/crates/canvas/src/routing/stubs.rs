//! Допустимые колонки первого и последнего отрезков (заглушек).

use super::model::Side;
use super::obstacles::{Obstacle, Own};
use super::model::U;

/// Предельный вылет заглушки от ближней допустимой колонки (клетки).
pub const MAX_STUB_REACH: i32 = 150;

/// Диапазон допустимых целых колонок `(от, до)` включительно для заглушки.
///
/// `own` — метка своей карточки (она заглушку не блокирует), `edge` — вертикаль карточки
/// на этой стороне, `y` — высота порта (субклетки), `window` — необязательное окно по X.
pub fn columns(
    obs: &[Obstacle],
    own: Own,
    edge: i32,
    side: Side,
    y: i32,
    min_stub: i32,
    window: Option<(i32, i32)>,
) -> Option<(i32, i32)> {
    let (mut lo, mut hi);
    match side {
        Side::Right => {
            let near = edge + min_stub;
            let mut far = near + MAX_STUB_REACH;
            for o in obs {
                if o.own != own && o.y0 * U < y && y < o.y1 * U && o.x1 > edge {
                    far = far.min(o.x0);
                }
            }
            lo = near;
            hi = far;
        }
        Side::Left => {
            let near = edge - min_stub;
            let mut far = near - MAX_STUB_REACH;
            for o in obs {
                if o.own != own && o.y0 * U < y && y < o.y1 * U && o.x0 < edge {
                    far = far.max(o.x1);
                }
            }
            lo = far;
            hi = near;
        }
    }
    if let Some((wl, wr)) = window {
        lo = lo.max(wl);
        hi = hi.min(wr);
    }
    if lo > hi {
        None
    } else {
        Some((lo, hi))
    }
}

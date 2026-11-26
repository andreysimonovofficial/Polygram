//! Препятствия и проверки свободы отрезков.
//! Границы препятствий в клетках, отрезки в субклетках; все неравенства строгие
//! (касание границы разрешено).

use super::model::{union_rect, Terminal, U};
use polygram_core::layout::Rect;

/// Дальность влияния (клетки): наибольшее поле окна 120 + предельный вылет заглушки 150.
pub const REACH: i32 = 270;

/// Чья это карточка относительно маршрута.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Own {
    /// Карточка источника.
    Src,
    /// Карточка цели.
    Dst,
    /// Любая другая.
    Other,
}

/// Препятствие: карточка, раздутая на охранную зону (клетки).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Obstacle {
    /// Левая граница.
    pub x0: i32,
    /// Верхняя граница.
    pub y0: i32,
    /// Правая граница.
    pub x1: i32,
    /// Нижняя граница.
    pub y1: i32,
    /// Метка принадлежности.
    pub own: Own,
}

/// Построить препятствия: берутся карточки, строго пересекающие рамку пары, расширенную на
/// `REACH`; каждая раздувается на `clear` клеток. Метка определяется по идентификатору карточки.
pub fn build(cards: &[(u32, Rect)], src: &Terminal, dst: &Terminal, clear: i32) -> Vec<Obstacle> {
    let pair = union_rect(&src.rect, &dst.rect);
    let frame = Rect::new(pair.x - REACH, pair.y - REACH, pair.w + 2 * REACH, pair.h + 2 * REACH);
    let mut out = Vec::new();
    for (id, r) in cards {
        if !frame.overlaps(r) {
            continue;
        }
        let own = if *id == src.card {
            Own::Src
        } else if *id == dst.card {
            Own::Dst
        } else {
            Own::Other
        };
        out.push(Obstacle {
            x0: r.x - clear,
            y0: r.y - clear,
            x1: r.x + r.w + clear,
            y1: r.y + r.h + clear,
            own,
        });
    }
    out
}

/// Вертикаль на колонке `x` (клетки) между высотами `ya` и `yb` (субклетки) свободна,
/// если ни одно препятствие (с любой меткой) не заходит на неё строго.
pub fn v_free(obs: &[Obstacle], x: i32, ya: i32, yb: i32) -> bool {
    let (lo, hi) = (ya.min(yb), ya.max(yb));
    !obs.iter()
        .any(|o| o.x0 < x && x < o.x1 && o.y0 * U < hi && lo < o.y1 * U)
}

/// Горизонталь на высоте `y` (субклетки) между `xa` и `xb` (субклетки) свободна,
/// если её не задевает ни одно препятствие, кроме меток из `skip`.
pub fn h_free(obs: &[Obstacle], y: i32, xa: i32, xb: i32, skip: &[Own]) -> bool {
    let (lo, hi) = (xa.min(xb), xa.max(xb));
    !obs.iter().any(|o| {
        !skip.contains(&o.own) && o.y0 * U < y && y < o.y1 * U && o.x0 * U < hi && lo < o.x1 * U
    })
}

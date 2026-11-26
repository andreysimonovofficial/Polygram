//! Маршрутизатор: ступени качества, область влияния, запасная ломаная.

use polygram_core::layout::Rect;

use super::direct::{self, Candidate};
use super::model::{manhattan, simplify, union_rect, Pt, Quality, Route, Side, Terminal, U};
use super::obstacles;
use super::search::{search, MARGINS};

/// Ступени качества по порядку: (охранная зона, минимальная заглушка, качество).
pub const TIERS: [(i32, i32, Quality); 3] = [
    (1, 2, Quality::Clean),
    (0, 2, Quality::Tight),
    (0, 1, Quality::Short),
];

/// Нижняя граница длины (субклетки): наименьшее манхэттенское расстояние между портами.
pub fn lower_bound(src: &Terminal, dst: &Terminal) -> i32 {
    let mut lb = i32::MAX;
    for s in Side::BOTH {
        for t in Side::BOTH {
            lb = lb.min(manhattan(src.port(s), dst.port(t)));
        }
    }
    lb
}

/// Решение одной ступени качества. `None` — пути нет.
pub fn solve(
    obs: &[obstacles::Obstacle],
    src: &Terminal,
    dst: &Terminal,
    min_stub: i32,
) -> Option<Candidate> {
    let lb = lower_bound(src, dst) as i64;
    let direct = direct::best(obs, src, dst, min_stub);
    if let Some(d) = &direct {
        if d.len == lb {
            return direct;
        }
    }
    let mut best = direct;
    for m in MARGINS {
        if let Some(c) = search(obs, src, dst, min_stub, m, false) {
            if best.as_ref().map_or(true, |b| c.cost < b.cost) {
                best = Some(c);
            }
        }
        if let Some(b) = &best {
            if b.len <= lb + 2 * m as i64 * U as i64 {
                break;
            }
        }
    }
    best
}

/// Построить маршрут между двумя терминалами среди карточек `cards` (идентификатор, прямоугольник).
/// Всегда возвращает маршрут: если ни одна ступень не нашла путь, строится запасная ломаная.
pub fn route(cards: &[(u32, Rect)], src: &Terminal, dst: &Terminal) -> Route {
    let lb = lower_bound(src, dst) as i64;
    for (clear, stub, quality) in TIERS {
        let obs = obstacles::build(cards, src, dst, clear);
        if let Some(c) = solve(&obs, src, dst, stub) {
            let slack = if quality == Quality::Clean {
                ((c.len - lb).max(0) / 2 / U as i64) as i32 + 4
            } else {
                300
            };
            return Route {
                src_side: c.src_side,
                dst_side: c.dst_side,
                points: c.points,
                quality,
                region: region(src, dst, slack),
            };
        }
    }
    fallback(src, dst)
}

/// Область влияния: рамка карточек и портов (клетки), расширенная на запас.
pub fn region(src: &Terminal, dst: &Terminal, slack: i32) -> Rect {
    let u = union_rect(&src.rect, &dst.rect);
    let (mut x0, mut y0, mut x1, mut y1) = (u.x, u.y, u.x + u.w, u.y + u.h);
    for p in [src.left, src.right, dst.left, dst.right] {
        let (cx, cy) = (p.x.div_euclid(U), p.y.div_euclid(U));
        x0 = x0.min(cx);
        y0 = y0.min(cy);
        x1 = x1.max(cx + 1);
        y1 = y1.max(cy + 1);
    }
    Rect::new(x0 - slack, y0 - slack, (x1 - x0) + 2 * slack, (y1 - y0) + 2 * slack)
}

/// Запасная ломаная: источник Правая, цель Левая, по колонкам на 2 клетки от карточек.
/// Ортогональна и существует всегда, но может пересекать карточки.
pub fn fallback(src: &Terminal, dst: &Terminal) -> Route {
    let (ps, pt) = (src.port(Side::Right), dst.port(Side::Left));
    let vs = (src.rect.x + src.rect.w + 2) * U;
    let vt = (dst.rect.x - 2) * U;
    let ym = ((ps.y + pt.y) / 2 / U) * U;
    let points = simplify(vec![
        ps,
        Pt::new(vs, ps.y),
        Pt::new(vs, ym),
        Pt::new(vt, ym),
        Pt::new(vt, pt.y),
        pt,
    ]);
    Route {
        src_side: Side::Right,
        dst_side: Side::Left,
        points,
        quality: Quality::Fallback,
        region: region(src, dst, 300),
    }
}

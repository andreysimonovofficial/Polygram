//! Точка входа маршрутизатора: подобрать линию связи между двумя показателями.
//!
//! Порядок: сначала идеальные варианты без поиска (прямая или «Z» по общей вертикали);
//! если они не самые короткие из возможных — поиск по сетке в растущем окне.
//! Если с «воздухом» вокруг карточек пути нет, воздух убирается, затем сокращается
//! первый/последний отрезок; в самом конце рисуется простая ломаная без обхода.

use super::cost::Cand;
use super::direct;
use super::model::{simplify, Params, Quality, Route, Side, Terminal, U};
use super::obstacles::{self, Obs, Win};
use super::search::search;
use super::stubs::MAX_REACH;
use polygram_core::layout::Rect;

/// Поля окна поиска вокруг карточек (клеток): пробуем от малых к большим.
const MARGINS: [i32; 4] = [4, 12, 40, 120];

/// Ступени качества: от строгой к мягкой.
const TIERS: [(Params, Quality); 3] = [
    (
        Params {
            clear: 1,
            min_stub: 2,
        },
        Quality::Clear,
    ),
    (
        Params {
            clear: 0,
            min_stub: 2,
        },
        Quality::Tight,
    ),
    (
        Params {
            clear: 0,
            min_stub: 1,
        },
        Quality::Short,
    ),
];

/// Проложить линию от `src` (выход) к `dst` (вход) в обход `obstacles` (все карточки).
pub fn route(src: &Terminal, dst: &Terminal, obstacles: &[Rect]) -> Route {
    for (par, quality) in TIERS {
        if let Some(c) = solve(src, dst, obstacles, par) {
            // С «воздухом» область влияния ограничена длиной пути; при уступках — широкая.
            let slack = if quality == Quality::Clear {
                ((c.len() as i64 - lower_bound(src, dst)).max(0) / 2 / U as i64) as i32 + 4
            } else {
                FAR_REGION
            };
            let region = region(src, dst, slack);
            return Route {
                src: c.src,
                dst: c.dst,
                points: c.points,
                quality,
                region,
            };
        }
    }
    fallback(src, dst)
}

/// Запас области влияния для маршрутов, найденных с уступками (клеток).
const FAR_REGION: i32 = 300;

/// Рамка вокруг обеих карточек и всех портов, расширенная на `slack` клеток.
fn region(src: &Terminal, dst: &Terminal, slack: i32) -> (i32, i32, i32, i32) {
    let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for t in [src, dst] {
        x0 = x0.min(t.rect.x);
        y0 = y0.min(t.rect.y);
        x1 = x1.max(t.rect.x + t.rect.w);
        y1 = y1.max(t.rect.y + t.rect.h);
        for s in Side::BOTH {
            let (px, py) = t.port_u(s);
            x0 = x0.min(px.div_euclid(U));
            x1 = x1.max(px.div_euclid(U) + 1);
            y0 = y0.min(py.div_euclid(U));
            y1 = y1.max(py.div_euclid(U) + 1);
        }
    }
    (x0 - slack, y0 - slack, x1 + slack, y1 + slack)
}

/// Кратчайший путь при заданных параметрах.
pub fn solve(src: &Terminal, dst: &Terminal, obstacles: &[Rect], par: Params) -> Option<Cand> {
    let reach = MARGINS[MARGINS.len() - 1] + MAX_REACH;
    let obs = obstacles::build(obstacles, &src.rect, &dst.rect, par, reach);

    let direct = direct::best(src, dst, &obs, par);
    let lb = lower_bound(src, dst);
    // Прямой вариант с минимально возможной длиной — оптимален, поиск не нужен.
    if let Some(d) = &direct {
        if d.len() as i64 == lb {
            return Some(d.clone());
        }
    }

    let mut best: Option<Cand> = direct;
    for margin in MARGINS {
        let win = window(src, dst, par, margin);
        if let Some(c) = search(src, dst, &obs, par, win) {
            let len = c.len() as i64;
            if best.as_ref().map_or(true, |b| c.cost < b.cost) {
                best = Some(c);
            }
            // Любой путь, вышедший за окно, длиннее lb + 2·margin: этот — точно кратчайший.
            let proven = best.as_ref().map_or(false, |b| {
                (b.len() as i64) <= lb + 2 * margin as i64 * U as i64
            });
            if proven || len == lb {
                break;
            }
        }
    }
    best
}

/// Нижняя граница длины любого пути (субклетки): манхэттен между ближайшими портами.
fn lower_bound(src: &Terminal, dst: &Terminal) -> i64 {
    let mut lb = i64::MAX;
    for s in Side::BOTH {
        for t in Side::BOTH {
            let (sx, sy) = src.port_u(s);
            let (tx, ty) = dst.port_u(t);
            lb = lb.min(((sx - tx).abs() + (sy - ty).abs()) as i64);
        }
    }
    lb
}

fn window(src: &Terminal, dst: &Terminal, par: Params, margin: i32) -> Win {
    let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for t in [src, dst] {
        let o = Obs::new(&t.rect, par.clear, obstacles::Own::Other);
        x0 = x0.min(o.x0);
        y0 = y0.min(o.y0);
        x1 = x1.max(o.x1);
        y1 = y1.max(o.y1);
        for s in Side::BOTH {
            let (px, py) = t.port_u(s);
            x0 = x0.min(px.div_euclid(U));
            x1 = x1.max(px.div_euclid(U) + 1);
            y0 = y0.min(py.div_euclid(U));
            y1 = y1.max(py.div_euclid(U) + 1);
        }
    }
    Win {
        x0: x0 - margin,
        y0: y0 - margin,
        x1: x1 + margin,
        y1: y1 + margin,
    }
}

/// Последнее средство: пути нет вообще (карточка зажата со всех сторон).
/// Простая ортогональная ломаная «выход справа → вход слева» без обхода препятствий.
fn fallback(src: &Terminal, dst: &Terminal) -> Route {
    let (psx, psy) = src.port_u(Side::Right);
    let (ptx, pty) = dst.port_u(Side::Left);
    let vs = (src.edge_x(Side::Right) + 2) * U;
    let vt = (dst.edge_x(Side::Left) - 2) * U;
    let ym = ((psy + pty) / 2 / U) * U;
    let points = simplify(vec![
        (psx, psy),
        (vs, psy),
        (vs, ym),
        (vt, ym),
        (vt, pty),
        (ptx, pty),
    ]);
    let region = region(src, dst, FAR_REGION);
    Route {
        src: Side::Right,
        dst: Side::Left,
        points,
        quality: Quality::Fallback,
        region,
    }
}

//! Поиск кратчайшего пути по линиям сетки в обход карточек (A* по «сжатой» сетке).
//!
//! Вся ломаная состоит из:
//!   1. первого отрезка — горизонталь от порта до вертикали сетки вне карточки;
//!   2. «ножки» — вертикали от конца первого отрезка до ближайшего пересечения линий сетки;
//!   3. пути по линиям сетки (вертикали и горизонтали), обходящего карточки;
//!   4. «ножки» вертикали к концу последнего отрезка;
//!   5. последнего отрезка — горизонталь до порта приёмника.
//!
//! Сетка «сжата»: вершины стоят только на линиях, где путь может повернуть с пользой —
//! границы карточек (с охранной зоной), ряды у портов, ближайшие допустимые вертикали
//! первого/последнего отрезка и середина между карточками. Любой кратчайший путь с
//! наименьшим числом колен можно сдвинуть на такие линии, не меняя ни длины, ни колен, —
//! поэтому результат тот же, что на полной сетке, а работы в десятки раз меньше.
//! (Это проверяет тест `router_is_never_worse_than_brute_force`.)
//!
//! Состояние хранит направление движения, поэтому разворот на 180° (петля, наложение
//! линии на себя) невозможен в принципе, а колена считаются точно.

use super::cost::{Cand, BEND, LEN, NONNATURAL, TIE_DIV};
use super::direct::mid;
use super::model::{simplify, Params, Side, Terminal, U};
use super::obstacles::{v_free, Obs, Own, Win};
use super::stubs::columns;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

const R: usize = 0;
const L: usize = 1;
const D: usize = 2;
const UP: usize = 3;
const NO_GOAL: u32 = u32::MAX;
const START: u32 = 0x8000_0000;

struct Start {
    side: Side,
    v: i32,
    psx: i32,
    psy: i32,
}

struct Goal {
    side: Side,
    v: i32,
    ptx: i32,
    pty: i32,
    /// Длина «ножки» от вершины сетки до конца последнего отрезка (знак — направление по Y).
    stub: i32,
    /// Направление последнего отрезка (к порту).
    last_dir: usize,
}

/// Линии сжатой сетки.
struct Lines {
    xs: Vec<i32>,
    ys: Vec<i32>,
}

impl Lines {
    fn build(src: &Terminal, dst: &Terminal, obs: &[Obs], par: Params, win: Win) -> Self {
        let (xmid, ymid) = mid(src, dst);
        let mut xs = vec![win.x0, win.x1, xmid.div_euclid(U), xmid.div_euclid(U) + 1];
        let mut ys = vec![win.y0, win.y1, ymid.div_euclid(U), ymid.div_euclid(U) + 1];
        for o in obs {
            xs.extend([o.x0, o.x1]);
            ys.extend([o.y0, o.y1]);
        }
        for t in [src, dst] {
            for s in Side::BOTH {
                let sign = if s == Side::Right { 1 } else { -1 };
                xs.push(t.edge_x(s) + sign * par.min_stub);
                let (_, py) = t.port_u(s);
                ys.extend([py.div_euclid(U), py.div_euclid(U) + 1]);
            }
        }
        xs.retain(|&x| x >= win.x0 && x <= win.x1);
        ys.retain(|&y| y >= win.y0 && y <= win.y1);
        xs.sort_unstable();
        xs.dedup();
        ys.sort_unstable();
        ys.dedup();
        Self { xs, ys }
    }

    fn col(&self, x: i32) -> Option<usize> {
        self.xs.binary_search(&x).ok()
    }

    fn row(&self, y: i32) -> Option<usize> {
        self.ys.binary_search(&y).ok()
    }
}

/// Занятые рёбра сжатой сетки.
struct Blocked {
    nx: usize,
    /// (a,b)-(a+1,b)
    h: Vec<bool>,
    /// (a,b)-(a,b+1)
    v: Vec<bool>,
}

impl Blocked {
    fn new(lines: &Lines, obs: &[Obs]) -> Self {
        let (nx, ny) = (lines.xs.len(), lines.ys.len());
        let mut b = Self {
            nx,
            h: vec![false; nx * ny],
            v: vec![false; nx * ny],
        };
        let pp = |v: &[i32], f: &dyn Fn(i32) -> bool| v.partition_point(|&x| f(x));
        for o in obs {
            // Горизонтальные рёбра: ряд строго внутри по Y, ребро пересекает (x0, x1) по X.
            let rows = pp(&lines.ys, &|y| y <= o.y0)..pp(&lines.ys, &|y| y < o.y1);
            let a_lo = pp(&lines.xs, &|x| x <= o.x0).saturating_sub(1);
            let a_hi = pp(&lines.xs, &|x| x < o.x1)
                .saturating_sub(1)
                .min(nx.saturating_sub(2));
            for r in rows {
                for a in a_lo..=a_hi {
                    if a + 1 < nx && lines.xs[a + 1] > o.x0 && lines.xs[a] < o.x1 {
                        b.h[r * nx + a] = true;
                    }
                }
            }
            // Вертикальные рёбра: колонка строго внутри по X, ребро пересекает (y0, y1) по Y.
            let cols = pp(&lines.xs, &|x| x <= o.x0)..pp(&lines.xs, &|x| x < o.x1);
            let r_lo = pp(&lines.ys, &|y| y <= o.y0).saturating_sub(1);
            let r_hi = pp(&lines.ys, &|y| y < o.y1)
                .saturating_sub(1)
                .min(ny.saturating_sub(2));
            for a in cols {
                for r in r_lo..=r_hi {
                    if r + 1 < ny && lines.ys[r + 1] > o.y0 && lines.ys[r] < o.y1 {
                        b.v[r * nx + a] = true;
                    }
                }
            }
        }
        b
    }
}

/// Один проход поиска в окне `win`.
pub fn search(src: &Terminal, dst: &Terminal, obs: &[Obs], par: Params, win: Win) -> Option<Cand> {
    // В сетку попадают только карточки, пересекающие окно.
    let local: Vec<Obs> = obs
        .iter()
        .copied()
        .filter(|o| o.x0 < win.x1 && o.x1 > win.x0 && o.y0 < win.y1 && o.y1 > win.y0)
        .collect();
    let lines = Lines::build(src, dst, &local, par, win);
    let blocked = Blocked::new(&lines, &local);
    let (nx, ny) = (lines.xs.len(), lines.ys.len());
    let (xmid, ymid) = mid(src, dst);
    let clamp = Some((win.x0, win.x1));
    let state = |a: usize, b: usize, d: usize| (b * nx + a) * 4 + d;

    let mut dist = vec![u64::MAX; nx * ny * 4];
    let mut parent = vec![u32::MAX; nx * ny * 4];
    let mut starts: Vec<Start> = Vec::new();
    let mut goals: Vec<Goal> = Vec::new();
    let mut goal_at: HashMap<(usize, usize), Vec<u32>> = HashMap::new();
    let mut heap: BinaryHeap<Reverse<(u64, u64, u32, u32)>> = BinaryHeap::new();

    // --- Старты: первый отрезок + «ножка» к линии сетки. ---
    for s in Side::BOTH {
        let (psx, psy) = src.port_u(s);
        let Some((lo, hi)) = columns(obs, Own::Src, src, s, psy, par, clamp) else {
            continue;
        };
        let out_dir = if s == Side::Right { R } else { L };
        let nonnat = if s == Side::Right { 0 } else { NONNATURAL };
        let rows: Vec<(i32, usize)> = if psy % U == 0 {
            vec![(psy / U, out_dir)]
        } else {
            let j0 = psy.div_euclid(U);
            vec![(j0, UP), (j0 + 1, D)]
        };
        let a0 = lines.xs.partition_point(|&x| x < lo);
        for a in a0..nx {
            let v = lines.xs[a];
            if v > hi {
                break;
            }
            let first = (v * U - psx).unsigned_abs() as u64;
            for &(j, dir) in &rows {
                let Some(b) = lines.row(j) else { continue };
                let (len, bends) = if dir == out_dir {
                    (first, 0)
                } else {
                    if !v_free(obs, v, psy, j * U) {
                        continue;
                    }
                    (first + (j * U - psy).unsigned_abs() as u64, 1)
                };
                let g = len * LEN + bends * BEND + nonnat;
                let st = state(a, b, dir);
                if g < dist[st] {
                    dist[st] = g;
                    parent[st] = START | starts.len() as u32;
                    starts.push(Start {
                        side: s,
                        v,
                        psx,
                        psy,
                    });
                    heap.push(Reverse((g + heuristic(dst, v, j), g, st as u32, NO_GOAL)));
                }
            }
        }
    }

    // --- Финиши: вертикаль у приёмника + «ножка» + последний отрезок. ---
    for t in Side::BOTH {
        let (ptx, pty) = dst.port_u(t);
        let Some((lo, hi)) = columns(obs, Own::Dst, dst, t, pty, par, clamp) else {
            continue;
        };
        let last_dir = if t == Side::Left { R } else { L };
        let js: Vec<i32> = if pty % U == 0 {
            vec![pty / U]
        } else {
            vec![pty.div_euclid(U), pty.div_euclid(U) + 1]
        };
        let a0 = lines.xs.partition_point(|&x| x < lo);
        for a in a0..nx {
            let v = lines.xs[a];
            if v > hi {
                break;
            }
            for &j in &js {
                let Some(b) = lines.row(j) else { continue };
                if !v_free(obs, v, j * U, pty) {
                    continue;
                }
                let id = goals.len() as u32;
                goals.push(Goal {
                    side: t,
                    v,
                    ptx,
                    pty,
                    stub: pty - j * U,
                    last_dir,
                });
                goal_at.entry((a, b)).or_default().push(id);
            }
        }
    }
    if starts.is_empty() || goals.is_empty() {
        return None;
    }

    // --- A* ---
    while let Some(Reverse((key, g, st, goal))) = heap.pop() {
        if goal != NO_GOAL {
            let gl = &goals[goal as usize];
            return Some(rebuild(key, st as usize, gl, &parent, &starts, &lines));
        }
        let st = st as usize;
        if g > dist[st] {
            continue;
        }
        let d = st % 4;
        let cell = st / 4;
        let (a, b) = (cell % nx, cell / nx);
        let (x, y) = (lines.xs[a], lines.ys[b]);

        // Подход к финишу из этой вершины.
        if let Some(ids) = goal_at.get(&(a, b)) {
            for &gid in ids {
                let gl = &goals[gid as usize];
                if let Some(extra) = finish_cost(gl, d, x) {
                    let nonnat = if gl.side == Side::Left { 0 } else { NONNATURAL };
                    heap.push(Reverse((g + extra + nonnat, g, st as u32, gid)));
                }
            }
        }

        for nd in 0..4 {
            if nd == (d ^ 1) {
                continue; // разворот на месте запрещён
            }
            let (na, nb, cells) = match nd {
                R if a + 1 < nx && !blocked.h[b * nx + a] => (a + 1, b, lines.xs[a + 1] - x),
                L if a > 0 && !blocked.h[b * nx + a - 1] => (a - 1, b, x - lines.xs[a - 1]),
                D if b + 1 < ny && !blocked.v[b * nx + a] => (a, b + 1, lines.ys[b + 1] - y),
                UP if b > 0 && !blocked.v[(b - 1) * nx + a] => (a, b - 1, y - lines.ys[b - 1]),
                _ => continue,
            };
            let tie = if nd == R || nd == L {
                ((y * U - ymid).abs() / TIE_DIV) as u64 * cells as u64
            } else {
                ((x * U - xmid).abs() / TIE_DIV) as u64 * cells as u64
            };
            let bend = if nd == d { 0 } else { BEND };
            let ng = g + (cells * U) as u64 * LEN + bend + tie;
            let ns = state(na, nb, nd);
            if ng < dist[ns] {
                dist[ns] = ng;
                parent[ns] = st as u32;
                let h = heuristic(dst, lines.xs[na], lines.ys[nb]);
                heap.push(Reverse((ng + h, ng, ns as u32, NO_GOAL)));
            }
        }
    }
    None
}

/// Нижняя оценка оставшейся длины от вершины сетки `(x, y)` (в клетках): манхэттенское
/// расстояние до ближайшего порта приёмника.
fn heuristic(dst: &Terminal, x: i32, y: i32) -> u64 {
    Side::BOTH
        .iter()
        .map(|&t| {
            let (px, py) = dst.port_u(t);
            ((x * U - px).abs() + (y * U - py).abs()) as u64
        })
        .min()
        .unwrap_or(0)
        * LEN
}

/// Цена от вершины (в которую пришли по направлению `d`, x — её вертикаль в клетках)
/// до порта приёмника. `None` — если пришлось бы развернуться.
fn finish_cost(gl: &Goal, d: usize, x_cells: i32) -> Option<u64> {
    let last = (x_cells * U - gl.ptx).unsigned_abs() as u64;
    if gl.stub == 0 {
        // Конец последнего отрезка лежит прямо на вершине сетки.
        if d == gl.last_dir {
            return Some(last * LEN);
        }
        if d == (gl.last_dir ^ 1) {
            return None;
        }
        return Some(last * LEN + BEND);
    }
    let stub_dir = if gl.stub > 0 { D } else { UP };
    let turn = if d == stub_dir {
        0
    } else if d == (stub_dir ^ 1) {
        return None;
    } else {
        BEND
    };
    // Ещё одно колено — из «ножки» в последний отрезок.
    Some((gl.stub.unsigned_abs() as u64 + last) * LEN + turn + BEND)
}

fn rebuild(
    cost: u64,
    last_state: usize,
    goal: &Goal,
    parent: &[u32],
    starts: &[Start],
    lines: &Lines,
) -> Cand {
    let nx = lines.xs.len();
    let mut verts: Vec<(i32, i32)> = Vec::new();
    let mut st = last_state;
    let start = loop {
        let cell = st / 4;
        verts.push((lines.xs[cell % nx] * U, lines.ys[cell / nx] * U));
        let p = parent[st];
        if p & START != 0 {
            break &starts[(p & !START) as usize];
        }
        st = p as usize;
    };
    verts.reverse();
    let mut pts = vec![(start.psx, start.psy), (start.v * U, start.psy)];
    pts.extend(verts);
    pts.push((goal.v * U, goal.pty));
    pts.push((goal.ptx, goal.pty));
    Cand::new(cost, simplify(pts), start.side, goal.side)
}

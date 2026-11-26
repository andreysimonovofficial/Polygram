//! Поиск A* по сжатой сетке. Состояние — (колонка, ряд, направление движения).
//! Разворот на 180° запрещён, поэтому колена считаются точно, а петли невозможны.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

use super::cost::{BEND_W, LEN_W, NONNATURAL, TIE_DIV};
use super::direct::{mid, Candidate};
use super::model::{manhattan, simplify, Pt, Side, Terminal, U};
use super::obstacles::{v_free, Obstacle, Own};
use super::stubs::columns;

/// Поля окна поиска (клетки), пробуются по очереди.
pub const MARGINS: [i32; 4] = [4, 12, 40, 120];

const RIGHT: usize = 0;
const LEFT: usize = 1;
const DOWN: usize = 2;
const UP: usize = 3;
const NONE: u32 = u32::MAX;

struct Goal {
    a: usize,
    b: usize,
    side: Side,
    /// Знаковая длина ножки: высота порта − ряд × 16.
    leg: i32,
    last_dir: usize,
    last_len: i32,
}

/// Поиск пути в окне с заданным полем. `dense = true` строит полную сетку из всех целых линий
/// (нужно только тестам для сверки со сжатой сеткой).
pub fn search(
    obs: &[Obstacle],
    src: &Terminal,
    dst: &Terminal,
    min_stub: i32,
    margin: i32,
    dense: bool,
) -> Option<Candidate> {
    // --- Окно ---
    let (mut wx0, mut wy0, mut wx1, mut wy1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for o in obs.iter().filter(|o| o.own != Own::Other) {
        wx0 = wx0.min(o.x0);
        wy0 = wy0.min(o.y0);
        wx1 = wx1.max(o.x1);
        wy1 = wy1.max(o.y1);
    }
    for p in [src.left, src.right, dst.left, dst.right] {
        let (cx, cy) = (p.x.div_euclid(U), p.y.div_euclid(U));
        wx0 = wx0.min(cx);
        wy0 = wy0.min(cy);
        wx1 = wx1.max(cx + 1);
        wy1 = wy1.max(cy + 1);
    }
    if wx0 > wx1 || wy0 > wy1 {
        return None;
    }
    wx0 -= margin;
    wy0 -= margin;
    wx1 += margin;
    wy1 += margin;

    let local: Vec<&Obstacle> = obs
        .iter()
        .filter(|o| o.x0 < wx1 && wx0 < o.x1 && o.y0 < wy1 && wy0 < o.y1)
        .collect();
    let (xmid, ymid) = mid(src, dst);

    // --- Линии сетки ---
    let (xs, ys): (Vec<i32>, Vec<i32>) = if dense {
        ((wx0..=wx1).collect(), (wy0..=wy1).collect())
    } else {
        let mut xs = vec![wx0, wx1, xmid.div_euclid(U), xmid.div_euclid(U) + 1];
        let mut ys = vec![wy0, wy1, ymid.div_euclid(U), ymid.div_euclid(U) + 1];
        for o in &local {
            xs.push(o.x0);
            xs.push(o.x1);
            ys.push(o.y0);
            ys.push(o.y1);
        }
        for t in [src, dst] {
            for s in Side::BOTH {
                xs.push(t.edge_x(s) + s.sign() * min_stub);
            }
            for p in [t.left, t.right] {
                ys.push(p.y.div_euclid(U));
                ys.push(p.y.div_euclid(U) + 1);
            }
        }
        xs.retain(|&x| wx0 <= x && x <= wx1);
        ys.retain(|&y| wy0 <= y && y <= wy1);
        xs.sort_unstable();
        xs.dedup();
        ys.sort_unstable();
        ys.dedup();
        (xs, ys)
    };
    let (nx, ny) = (xs.len(), ys.len());
    if nx < 2 || ny < 2 {
        return None;
    }

    // --- Занятые рёбра ---
    let mut hblock = vec![false; ny * (nx - 1)];
    let mut vblock = vec![false; (ny - 1) * nx];
    for o in &local {
        for b in 0..ny {
            if o.y0 < ys[b] && ys[b] < o.y1 {
                for a in 0..nx - 1 {
                    if xs[a + 1] > o.x0 && xs[a] < o.x1 {
                        hblock[b * (nx - 1) + a] = true;
                    }
                }
            }
        }
        for a in 0..nx {
            if o.x0 < xs[a] && xs[a] < o.x1 {
                for b in 0..ny - 1 {
                    if ys[b + 1] > o.y0 && ys[b] < o.y1 {
                        vblock[b * nx + a] = true;
                    }
                }
            }
        }
    }

    let n_states = nx * ny * 4;
    let st = |a: usize, b: usize, d: usize| (b * nx + a) * 4 + d;
    let mut dist = vec![u64::MAX; n_states];
    let mut parent = vec![NONE; n_states];
    let mut start_side: Vec<Option<Side>> = vec![None; n_states];
    let mut heap: BinaryHeap<Reverse<(u64, u64, u32, u32)>> = BinaryHeap::new();

    let heur = |a: usize, b: usize| -> u64 {
        let p = Pt::new(xs[a] * U, ys[b] * U);
        let m = manhattan(p, dst.left).min(manhattan(p, dst.right));
        m as u64 * LEN_W
    };

    // --- Старты: первый отрезок и ножка ---
    for s in Side::BOTH {
        let ps = src.port(s);
        let Some((lo, hi)) = columns(obs, Own::Src, src.edge_x(s), s, ps.y, min_stub, Some((wx0, wx1))) else {
            continue;
        };
        let out_dir = if s == Side::Right { RIGHT } else { LEFT };
        let nonnat = if s == Side::Left { NONNATURAL } else { 0 };
        let aligned = ps.y.rem_euclid(U) == 0;
        let rows: Vec<(i32, usize)> = if aligned {
            vec![(ps.y / U, out_dir)]
        } else {
            let f = ps.y.div_euclid(U);
            vec![(f, UP), (f + 1, DOWN)]
        };
        for a in 0..nx {
            if xs[a] < lo || xs[a] > hi {
                continue;
            }
            let first = (xs[a] * U - ps.x).abs();
            for &(row, dir) in &rows {
                let Ok(b) = ys.binary_search(&row) else { continue };
                let (len, bends) = if aligned {
                    (first, 0u64)
                } else {
                    if !v_free(obs, xs[a], ps.y, row * U) {
                        continue;
                    }
                    (first + (row * U - ps.y).abs(), 1u64)
                };
                let g = len as u64 * LEN_W + bends * BEND_W + nonnat;
                let sidx = st(a, b, dir);
                if g < dist[sidx] {
                    dist[sidx] = g;
                    parent[sidx] = NONE;
                    start_side[sidx] = Some(s);
                    heap.push(Reverse((g + heur(a, b), g, sidx as u32, NONE)));
                }
            }
        }
    }

    // --- Финиши: ножка и последний отрезок ---
    let mut goals: Vec<Goal> = Vec::new();
    let mut goals_at: HashMap<usize, Vec<usize>> = HashMap::new();
    for t in Side::BOTH {
        let pt = dst.port(t);
        let Some((lo, hi)) = columns(obs, Own::Dst, dst.edge_x(t), t, pt.y, min_stub, Some((wx0, wx1))) else {
            continue;
        };
        let last_dir = if t == Side::Left { RIGHT } else { LEFT };
        let rows: Vec<i32> = if pt.y.rem_euclid(U) == 0 {
            vec![pt.y / U]
        } else {
            let f = pt.y.div_euclid(U);
            vec![f, f + 1]
        };
        for a in 0..nx {
            if xs[a] < lo || xs[a] > hi {
                continue;
            }
            for &row in &rows {
                let Ok(b) = ys.binary_search(&row) else { continue };
                if !v_free(obs, xs[a], row * U, pt.y) {
                    continue;
                }
                goals_at.entry(b * nx + a).or_default().push(goals.len());
                goals.push(Goal {
                    a,
                    b,
                    side: t,
                    leg: pt.y - row * U,
                    last_dir,
                    last_len: (xs[a] * U - pt.x).abs(),
                });
            }
        }
    }
    if goals.is_empty() {
        return None;
    }

    // --- Основной цикл ---
    while let Some(Reverse((_f, g, s_idx, goal))) = heap.pop() {
        if goal != NONE {
            return Some(rebuild(
                s_idx as usize, &goals[goal as usize], g, &parent, &start_side, &xs, &ys, nx, src, dst,
            ));
        }
        let s_idx = s_idx as usize;
        if g > dist[s_idx] {
            continue;
        }
        let d = s_idx % 4;
        let vtx = s_idx / 4;
        let (a, b) = (vtx % nx, vtx / nx);

        if let Some(gs) = goals_at.get(&vtx) {
            for &gi in gs {
                if let Some(fc) = finish_cost(d, &goals[gi]) {
                    let tot = g + fc;
                    heap.push(Reverse((tot, tot, s_idx as u32, gi as u32)));
                }
            }
        }

        for nd in 0..4usize {
            if nd == (d ^ 1) {
                continue;
            }
            let (na, nb, cells, horizontal) = match nd {
                RIGHT if a + 1 < nx && !hblock[b * (nx - 1) + a] => (a + 1, b, xs[a + 1] - xs[a], true),
                LEFT if a > 0 && !hblock[b * (nx - 1) + a - 1] => (a - 1, b, xs[a] - xs[a - 1], true),
                DOWN if b + 1 < ny && !vblock[b * nx + a] => (a, b + 1, ys[b + 1] - ys[b], false),
                UP if b > 0 && !vblock[(b - 1) * nx + a] => (a, b - 1, ys[b] - ys[b - 1], false),
                _ => continue,
            };
            let tie = if horizontal {
                ((ys[b] * U - ymid).abs() / TIE_DIV) as u64
            } else {
                ((xs[a] * U - xmid).abs() / TIE_DIV) as u64
            } * cells as u64;
            let step = cells as u64 * U as u64 * LEN_W + if nd != d { BEND_W } else { 0 } + tie;
            let ng = g + step;
            let ns = st(na, nb, nd);
            if ng < dist[ns] {
                dist[ns] = ng;
                parent[ns] = s_idx as u32;
                heap.push(Reverse((ng + heur(na, nb), ng, ns as u32, NONE)));
            }
        }
    }
    None
}

/// Стоимость подхода к финишу из вершины при направлении прихода `d`.
fn finish_cost(d: usize, g: &Goal) -> Option<u64> {
    let nonnat = if g.side == Side::Right { NONNATURAL } else { 0 };
    let last = g.last_len as u64 * LEN_W;
    if g.leg == 0 {
        if d == g.last_dir {
            Some(last + nonnat)
        } else if d == (g.last_dir ^ 1) {
            None
        } else {
            Some(last + BEND_W + nonnat)
        }
    } else {
        let leg_dir = if g.leg > 0 { DOWN } else { UP };
        let into_leg = if d == leg_dir {
            0
        } else if d == (leg_dir ^ 1) {
            return None;
        } else {
            BEND_W
        };
        Some((g.leg.unsigned_abs() as u64 + g.last_len as u64) * LEN_W + into_leg + BEND_W + nonnat)
    }
}

#[allow(clippy::too_many_arguments)]
fn rebuild(
    goal_state: usize,
    goal: &Goal,
    total: u64,
    parent: &[u32],
    start_side: &[Option<Side>],
    xs: &[i32],
    ys: &[i32],
    nx: usize,
    src: &Terminal,
    dst: &Terminal,
) -> Candidate {
    let mut verts: Vec<Pt> = Vec::new();
    let mut cur = goal_state;
    loop {
        let vtx = cur / 4;
        verts.push(Pt::new(xs[vtx % nx] * U, ys[vtx / nx] * U));
        if parent[cur] == NONE {
            break;
        }
        cur = parent[cur] as usize;
    }
    verts.reverse();
    let side = start_side[cur].unwrap_or(Side::Right);
    let ps = src.port(side);
    let pt = dst.port(goal.side);
    let mut pts = vec![ps, Pt::new(verts[0].x, ps.y)];
    pts.extend(verts.iter().copied());
    pts.push(Pt::new(verts[verts.len() - 1].x, pt.y));
    pts.push(pt);
    let _ = goal.a;
    let _ = goal.b;
    let points = simplify(pts);
    let len: i64 = points
        .windows(2)
        .map(|w| ((w[1].x - w[0].x).abs() + (w[1].y - w[0].y).abs()) as i64)
        .sum();
    Candidate { src_side: side, dst_side: goal.side, points, cost: total, len }
}

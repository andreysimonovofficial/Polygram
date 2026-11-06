//! Маршрутизация связей: ортогональные линии (только горизонталь и вертикаль).
//!
//! Прямой случай (приёмник правее источника, между ними есть зазор):
//!   выход → по горизонтали в канал → по вертикали → по горизонтали → вход.
//! Канал — колонка клеток в зазоре; в каждой колонке 3 параллельные дорожки, чтобы линии
//! не слипались. Колонка выбирается так, чтобы линия не проходила сквозь чужие карточки.
//! Обратный случай (приёмник левее или вплотную): обход сверху или снизу, 5 отрезков.
//!
//! Ограничение v0: при перемещении объекта пересчитываются только его собственные связи.
//! Полный пересчёт — команда `reroute_all`.

use crate::{
    hierarchy::Hierarchy,
    layout::{Layout, Point, Rect, Q},
    types::*,
};
use std::collections::HashMap;

/// Число параллельных дорожек в одной колонке-канале.
pub const LANES: usize = 3;

#[derive(Clone, Debug, Default)]
pub struct LaneRegistry {
    cols: HashMap<i32, [Vec<(i32, i32, u32)>; LANES]>,
    by_edge: HashMap<u32, (i32, usize)>,
}

impl LaneRegistry {
    pub fn release(&mut self, e: u32) {
        if let Some((col, lane)) = self.by_edge.remove(&e) {
            if let Some(c) = self.cols.get_mut(&col) {
                c[lane].retain(|x| x.2 != e);
            }
        }
    }

    /// Занять дорожку в колонке `col` на отрезке по вертикали [y0, y1].
    pub fn reserve(&mut self, e: u32, col: i32, y0: i32, y1: i32) -> usize {
        self.release(e);
        let (lo, hi) = (y0.min(y1), y0.max(y1));
        let entry = self.cols.entry(col).or_default();
        let lane = (0..LANES)
            .find(|&l| entry[l].iter().all(|&(a, b, _)| hi < a || b < lo))
            .unwrap_or((e as usize) % LANES);
        entry[lane].push((lo, hi, e));
        self.by_edge.insert(e, (col, lane));
        lane
    }
}

fn simplify(points: Vec<Point>) -> Vec<Point> {
    let mut out: Vec<Point> = Vec::with_capacity(points.len());
    for p in points {
        if out.last() == Some(&p) {
            continue;
        }
        while out.len() >= 2 {
            let a = out[out.len() - 2];
            let b = out[out.len() - 1];
            let collinear = (a.x == b.x && b.x == p.x) || (a.y == b.y && b.y == p.y);
            if collinear {
                out.pop();
            } else {
                break;
            }
        }
        out.push(p);
    }
    out
}

impl Layout {
    pub fn release_route(&mut self, e: EdgeIx) {
        self.lanes.release(e.0);
        if let Some(r) = self.routes.get_mut(e.idx()) {
            r.clear();
        }
    }

    pub fn reset_lanes(&mut self) {
        self.lanes = LaneRegistry::default();
    }

    /// Сколько разных чужих карточек пересекают прямоугольники.
    fn blockers(&self, rects: &[Rect], src: ObjectIx, dst: ObjectIx) -> usize {
        let mut all: Vec<u32> = Vec::new();
        let mut tmp: Vec<u32> = Vec::new();
        for r in rects {
            self.objects_overlapping(r, None, &mut tmp);
            all.extend(tmp.iter().copied().filter(|&id| id != src.0 && id != dst.0));
        }
        all.sort_unstable();
        all.dedup();
        all.len()
    }

    /// Диапазон колонок-каналов [lo, hi], при котором горизонтали не упираются в чужие карточки.
    fn channel_range(
        &self,
        src: ObjectIx,
        dst: ObjectIx,
        srow: i32,
        drow: i32,
        sc: i32,
        dc: i32,
    ) -> (i32, i32) {
        let (mut lo, mut hi) = (sc, dc);
        let mut tmp: Vec<u32> = Vec::new();
        self.objects_overlapping(&Rect::new(sc, srow, dc - sc + 1, 1), None, &mut tmp);
        for &id in &tmp {
            if id != src.0 && id != dst.0 {
                hi = hi.min(self.rect[id as usize].x - 1);
            }
        }
        self.objects_overlapping(&Rect::new(sc, drow, dc - sc + 1, 1), None, &mut tmp);
        for &id in &tmp {
            if id != src.0 && id != dst.0 {
                let r = self.rect[id as usize];
                lo = lo.max(r.x + r.w);
            }
        }
        (lo, hi)
    }

    pub fn route_edge(&mut self, h: &Hierarchy, e: EdgeIx) {
        let ed = *h.edge(e);
        let (src, dst) = (
            h.object_metric(ed.from).object,
            h.object_metric(ed.to).object,
        );
        let (ks, kd) = (h.entry_position(ed.from), h.entry_position(ed.to));
        let path = self.compute_path(e, src, ks, dst, kd);
        self.routes[e.idx()] = path;
    }

    fn compute_path(
        &mut self,
        e: EdgeIx,
        src: ObjectIx,
        ks: usize,
        dst: ObjectIx,
        kd: usize,
    ) -> Vec<Point> {
        let a = self.rect_of(src);
        let b = self.rect_of(dst);
        let s = self.out_port(src, ks);
        let d = self.request_port(dst, kd);
        let srow = a.y + 1 + ks as i32;
        let drow = b.y + 1 + kd as i32;
        let sc = a.x + a.w; // первая клетка справа от источника
        let dc = b.x - 1; // последняя клетка слева от приёмника

        if dc >= sc {
            // Прямая линия, если строки совпадают и путь свободен.
            if srow == drow && self.blockers(&[Rect::new(sc, srow, dc - sc + 1, 1)], src, dst) == 0
            {
                self.lanes.release(e.0);
                return vec![s, d];
            }
            let mid = (sc + dc) / 2;
            // Допустимый диапазон колонок: горизонтали у источника и у приёмника не должны
            // упираться в чужие карточки. Если диапазон пуст — ищем лучшее из плохого.
            let (lo, hi) = self.channel_range(src, dst, srow, drow, sc, dc);
            let (lo, hi) = if lo <= hi { (lo, hi) } else { (sc, dc) };
            let center = mid.clamp(lo, hi);
            let span = hi - lo + 1;
            let mut best: Option<(usize, i32, i32)> = None; // (цена, расстояние до центра, колонка)
            let mut tried = 0;
            let mut k = 0i32;
            while tried < 12 && k <= 2 * span + 2 {
                let off = ((k + 1) / 2) * if k % 2 == 1 { 1 } else { -1 };
                k += 1;
                let c = center + off;
                if c < lo || c > hi {
                    continue;
                }
                tried += 1;
                let rects = [
                    Rect::new(sc, srow, c - sc + 1, 1),
                    Rect::new(c, srow.min(drow), 1, (srow - drow).abs() + 1),
                    Rect::new(c, drow, dc - c + 1, 1),
                ];
                let cost = self.blockers(&rects, src, dst);
                let cand = (cost, (c - center).abs(), c);
                if best.map_or(true, |bst| cand < bst) {
                    best = Some(cand);
                }
                if cost == 0 {
                    break;
                }
            }
            let col = best.map(|b| b.2).unwrap_or(mid);
            let lane = self.lanes.reserve(e.0, col, s.y, d.y) as i32;
            let x = col * Q + 1 + lane;
            return simplify(vec![s, Point { x, y: s.y }, Point { x, y: d.y }, d]);
        }

        // Обратная связь: обход сверху или снизу.
        self.lanes.release(e.0);
        let top = a.y.min(b.y) - 1;
        let bottom = (a.y + a.h).max(b.y + b.h);
        let up = (srow - top).abs() + (drow - top).abs();
        let down = (srow - bottom).abs() + (drow - bottom).abs();
        let rowb = if up <= down { top } else { bottom };
        let yb = rowb * Q + Q / 2;
        let lane = (e.0 as usize % LANES) as i32;
        let xa = sc * Q + 1 + lane;
        let xb = dc * Q + 1 + lane;
        simplify(vec![
            s,
            Point { x: xa, y: s.y },
            Point { x: xa, y: yb },
            Point { x: xb, y: yb },
            Point { x: xb, y: d.y },
            d,
        ])
    }

    /// Пересчитать маршруты всех помеченных связей. Возвращает пересчитанные.
    pub fn flush_routes(&mut self, h: &Hierarchy) -> Vec<EdgeIx> {
        let mut q = self.take_route_queue();
        q.sort_unstable();
        let mut done = Vec::with_capacity(q.len());
        for e in q {
            let e = EdgeIx(e);
            if e.idx() < h.edge_slots() && h.edge(e).alive {
                self.route_edge(h, e);
                done.push(e);
            }
        }
        done
    }
}

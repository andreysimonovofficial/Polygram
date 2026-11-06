//! Автораскладка: слева направо по «рангам» (кто кого питает), без наложений.
//!
//! 1. Ранг объекта = длина самой длинной цепочки от источников (циклы разрываются).
//! 2. Каждый ранг — колонка; между колонками зазор под линии.
//! 3. Внутри колонки объекты упорядочены по средней высоте своих поставщиков,
//!    чтобы линии пересекались реже.

use crate::{
    hierarchy::Hierarchy,
    layout::{footprint_for, Layout, Rect},
    types::*,
};
use std::collections::VecDeque;

/// Зазор между колонками (клеток): 3 колонки-канала по 3 дорожки.
const GAP_COLS: i32 = 3;
/// Зазор между карточками в колонке (клеток).
const GAP_ROWS: i32 = 1;

impl Layout {
    pub fn auto_layout(&mut self, h: &Hierarchy) {
        let n = h.object_slots();
        let alive: Vec<ObjectIx> = h.live_objects().collect();
        if alive.is_empty() {
            return;
        }

        let mut indeg = vec![0u32; n];
        let mut succ: Vec<Vec<u32>> = vec![Vec::new(); n];
        let mut preds: Vec<Vec<u32>> = vec![Vec::new(); n];
        for e in h.live_edges() {
            let d = h.edge(e);
            let a = h.object_metric(d.from).object.idx();
            let b = h.object_metric(d.to).object.idx();
            succ[a].push(b as u32);
            preds[b].push(a as u32);
            indeg[b] += 1;
        }

        // Ранги: самый длинный путь, циклы разрываются принудительно.
        let mut rank = vec![0u32; n];
        let mut done = vec![false; n];
        let mut queued = vec![false; n];
        let mut queue: VecDeque<u32> = VecDeque::new();
        for &x in &alive {
            if indeg[x.idx()] == 0 {
                queue.push_back(x.0);
                queued[x.idx()] = true;
            }
        }
        let mut processed = 0usize;
        let mut cursor = 0usize;
        loop {
            while let Some(u) = queue.pop_front() {
                let u = u as usize;
                done[u] = true;
                processed += 1;
                for i in 0..succ[u].len() {
                    let v = succ[u][i] as usize;
                    if done[v] {
                        continue;
                    }
                    rank[v] = rank[v].max(rank[u] + 1);
                    if indeg[v] > 0 {
                        indeg[v] -= 1;
                        if indeg[v] == 0 && !queued[v] {
                            queued[v] = true;
                            queue.push_back(v as u32);
                        }
                    }
                }
            }
            if processed == alive.len() {
                break;
            }
            while cursor < alive.len() && (done[alive[cursor].idx()] || queued[alive[cursor].idx()])
            {
                cursor += 1;
            }
            let v = alive[cursor].idx();
            indeg[v] = 0;
            queued[v] = true;
            queue.push_back(v as u32);
        }

        // Колонки.
        let max_rank = alive.iter().map(|x| rank[x.idx()]).max().unwrap_or(0) as usize;
        let mut cols: Vec<Vec<u32>> = vec![Vec::new(); max_rank + 1];
        for &x in &alive {
            cols[rank[x.idx()] as usize].push(x.0);
        }

        let mut center_y = vec![0f64; n];
        let mut placed_rect: Vec<Rect> = vec![Rect::new(0, 0, 0, 0); n];
        let mut col_x = 0i32;
        for (r, col) in cols.iter_mut().enumerate() {
            if r > 0 {
                let mut keyed: Vec<(f64, u32)> = col
                    .iter()
                    .map(|&v| {
                        let ps = &preds[v as usize];
                        let mut sum = 0.0;
                        let mut cnt = 0.0;
                        for &p in ps {
                            if rank[p as usize] < r as u32 {
                                sum += center_y[p as usize];
                                cnt += 1.0;
                            }
                        }
                        (if cnt > 0.0 { sum / cnt } else { f64::MAX }, v)
                    })
                    .collect();
                keyed.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.1.cmp(&b.1)));
                *col = keyed.into_iter().map(|k| k.1).collect();
            }
            let mut y = 0i32;
            for &v in col.iter() {
                let o = h.object(ObjectIx(v));
                let (w, hh) = footprint_for(o.requests.len(), o.outputs.len());
                placed_rect[v as usize] = Rect::new(col_x, y, w as i32, hh as i32);
                center_y[v as usize] = y as f64 + hh as f64 / 2.0;
                y += hh as i32 + GAP_ROWS;
            }
            col_x += crate::layout::NODE_W as i32 + GAP_COLS;
        }

        // Применить: снять всех, поставить заново.
        for &x in &alive {
            self.unplace(x);
        }
        for &x in &alive {
            self.place(x, placed_rect[x.idx()]);
        }
        self.reset_lanes();
        for e in h.live_edges() {
            self.queue_route(e);
        }
    }
}

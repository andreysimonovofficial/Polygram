//! Хранилище 3 — Архитектура графа: где что стоит на сетке и как идут линии.
//!
//! * Объекты привязаны к клеткам сетки (целые координаты), наложения запрещены.
//! * Размер карточки: ширина 8 клеток, высота = 1 (шапка) + строки показателей.
//! * Запросы — слева, выходы — справа; порт показателя номер k лежит на строке k.
//! * Линии считаются в «квантах» (1 клетка = 4 кванта), чтобы вести параллельные дорожки.
//!
//! Маршрутизация — в `route.rs`, автораскладка — в `autolayout.rs`.

use crate::{route::LaneRegistry, types::*};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Квантов линии в одной клетке.
pub const Q: i32 = 4;
/// Ширина карточки в клетках.
pub const NODE_W: u16 = 30;
/// Размер корзины пространственного индекса (в клетках).
const BUCKET: i32 = 16;
/// Шаг решётки автоподстановки (клеток): ширина и высота слота.
const AUTO_PITCH_X: i32 = 30;
const AUTO_PITCH_Y: i32 = 30;
const AUTO_COLUMNS: u32 = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }
    #[inline]
    pub fn overlaps(&self, o: &Rect) -> bool {
        self.x < o.x + o.w && o.x < self.x + self.w && self.y < o.y + o.h && o.y < self.y + self.h
    }

    /// Пересечение с буфером `buffer_quanta` квантов вокруг каждого прямоугольника.
    /// Возвращает true, если расстояние между контурами меньше `buffer_quanta`.
    #[inline]
    pub fn overlaps_with_buffer(&self, o: &Rect, buffer_quanta: i32) -> bool {
        let l1 = self.x * Q - buffer_quanta;
        let r1 = (self.x + self.w) * Q + buffer_quanta;
        let l2 = o.x * Q - buffer_quanta;
        let r2 = (o.x + o.w) * Q + buffer_quanta;
        let t1 = self.y * Q - buffer_quanta;
        let b1 = (self.y + self.h) * Q + buffer_quanta;
        let t2 = o.y * Q - buffer_quanta;
        let b2 = (o.y + o.h) * Q + buffer_quanta;
        l1 < r2 && l2 < r1 && t1 < b2 && t2 < b1
    }
}

/// Точка линии в квантах.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

/// Размер карточки в клетках по числу запросов и выходов.
pub fn footprint_for(n_req: usize, n_out: usize) -> (u16, u16) {
    let rows = n_req.max(n_out).max(29);
    (NODE_W, (1 + rows) as u16)
}

#[derive(Clone, Debug, Default)]
struct SpatialGrid {
    buckets: HashMap<(i32, i32), Vec<u32>>,
    /// id -> vec of (bucket, index_in_bucket) для O(1) удаления.
    positions: HashMap<u32, Vec<((i32, i32), usize)>>,
}

fn bucket_range(r: &Rect) -> (i32, i32, i32, i32) {
    (
        r.x.div_euclid(BUCKET),
        r.y.div_euclid(BUCKET),
        (r.x + r.w - 1).div_euclid(BUCKET),
        (r.y + r.h - 1).div_euclid(BUCKET),
    )
}

impl SpatialGrid {
    fn insert(&mut self, id: u32, r: &Rect) {
        let (bx0, by0, bx1, by1) = bucket_range(r);
        let mut recs = Vec::with_capacity(((bx1 - bx0 + 1) * (by1 - by0 + 1)) as usize);
        for by in by0..=by1 {
            for bx in bx0..=bx1 {
                let key = (bx, by);
                let v = self.buckets.entry(key).or_default();
                let idx = v.len();
                v.push(id);
                recs.push((key, idx));
            }
        }
        self.positions.insert(id, recs);
    }

    fn remove(&mut self, id: u32, _r: &Rect) {
        let recs = match self.positions.remove(&id) {
            Some(r) => r,
            None => return,
        };
        for (key, idx) in recs {
            let last = {
                let v = self.buckets.get_mut(&key).expect("bucket exists");
                let last = v.len() - 1;
                if idx != last {
                    let moved = v[last];
                    if let Some(mrecs) = self.positions.get_mut(&moved) {
                        for rec in mrecs.iter_mut() {
                            if rec.0 == key && rec.1 == last {
                                rec.1 = idx;
                                break;
                            }
                        }
                    }
                }
                last
            };
            {
                let v = self.buckets.get_mut(&key).expect("bucket exists");
                v.swap_remove(idx);
            }
            if self.buckets.get(&key).map_or(true, |v| v.is_empty()) {
                self.buckets.remove(&key);
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Layout {
    pub(crate) rect: Vec<Rect>,
    pub(crate) placed: Vec<bool>,
    grid: SpatialGrid,
    pub(crate) routes: Vec<Vec<Point>>,
    pub(crate) lanes: LaneRegistry,
    route_queue: Vec<u32>,
    route_queued: Vec<bool>,
    auto_slot: u32,
}

impl Layout {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ensure(&mut self, objects: usize, edges: usize) {
        if self.rect.len() < objects {
            self.rect.resize(objects, Rect::new(0, 0, 0, 0));
            self.placed.resize(objects, false);
        }
        if self.routes.len() < edges {
            self.routes.resize(edges, Vec::new());
            self.route_queued.resize(edges, false);
        }
    }

    #[inline]
    pub fn rect_of(&self, o: ObjectIx) -> Rect {
        self.rect[o.idx()]
    }

    #[inline]
    pub fn is_placed(&self, o: ObjectIx) -> bool {
        self.placed[o.idx()]
    }

    pub fn route(&self, e: EdgeIx) -> &[Point] {
        &self.routes[e.idx()]
    }

    /// Можно ли поставить прямоугольник (не пересекается ни с кем, кроме `ignore`).
    /// Между контурами объектов сохраняется буфер 2 кванта.
    pub fn is_free(&self, r: &Rect, ignore: Option<ObjectIx>) -> bool {
        // Расширяем диапазон корзин на 1 клетку, чтобы поймать объекты в соседние корзины,
        // которые могут быть в пределах буфера 2 кванта от r.
        let padded = Rect::new(r.x - 1, r.y - 1, r.w + 2, r.h + 2);
        let (bx0, by0, bx1, by1) = bucket_range(&padded);
        for by in by0..=by1 {
            for bx in bx0..=bx1 {
                if let Some(v) = self.grid.buckets.get(&(bx, by)) {
                    for &id in v {
                        if Some(ObjectIx(id)) == ignore {
                            continue;
                        }
                        if self.rect[id as usize].overlaps_with_buffer(r, 2) {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }

    /// Номера объектов, чьи карточки пересекают прямоугольник (без повторов, по возрастанию).
    /// Учитывается буфер 2 кванта между контурами.
    pub fn objects_overlapping(&self, r: &Rect, ignore: Option<ObjectIx>, out: &mut Vec<u32>) {
        out.clear();
        let padded = Rect::new(r.x - 1, r.y - 1, r.w + 2, r.h + 2);
        let (bx0, by0, bx1, by1) = bucket_range(&padded);
        for by in by0..=by1 {
            for bx in bx0..=bx1 {
                if let Some(v) = self.grid.buckets.get(&(bx, by)) {
                    for &id in v {
                        if Some(ObjectIx(id)) != ignore && self.rect[id as usize].overlaps_with_buffer(r, 2) {
                            out.push(id);
                        }
                    }
                }
            }
        }
        out.sort_unstable();
        out.dedup();
    }

    pub fn place(&mut self, o: ObjectIx, r: Rect) {
        debug_assert!(!self.placed[o.idx()]);
        self.rect[o.idx()] = r;
        self.placed[o.idx()] = true;
        self.grid.insert(o.0, &r);
    }

    pub fn unplace(&mut self, o: ObjectIx) {
        if self.placed[o.idx()] {
            let r = self.rect[o.idx()];
            self.grid.remove(o.0, &r);
            self.placed[o.idx()] = false;
        }
    }

    /// Переставить/изменить размер. Вызывающий обязан заранее проверить `is_free`.
    pub fn replace(&mut self, o: ObjectIx, r: Rect) {
        self.unplace(o);
        self.place(o, r);
    }

    /// Ближайшее свободное место (кольцами вокруг точки). Порядок обхода детерминирован.
    pub fn find_free_near(
        &self,
        w: i32,
        h: i32,
        cx: i32,
        cy: i32,
        ignore: Option<ObjectIx>,
        max_radius: i32,
    ) -> Option<(i32, i32)> {
        let free = |x: i32, y: i32| self.is_free(&Rect::new(x, y, w, h), ignore);
        if free(cx, cy) {
            return Some((cx, cy));
        }
        for r in 1..=max_radius {
            for dx in -r..=r {
                if free(cx + dx, cy - r) {
                    return Some((cx + dx, cy - r));
                }
                if free(cx + dx, cy + r) {
                    return Some((cx + dx, cy + r));
                }
            }
            for dy in (-r + 1)..r {
                if free(cx - r, cy + dy) {
                    return Some((cx - r, cy + dy));
                }
                if free(cx + r, cy + dy) {
                    return Some((cx + r, cy + dy));
                }
            }
        }
        None
    }

    /// Свободное место для rect w×h внутри rect родителя (отдела), ниже шапки.
    /// Ищет перебором от левого верхнего угла внутренней области.
    pub fn find_free_in_parent(&self, parent: ObjectIx, w: i32, h: i32) -> Option<(i32, i32)> {
        let pr = self.rect_of(parent);
        let inner_y = pr.y + 2;
        let inner_h = pr.h.saturating_sub(2);
        let max_y = (inner_y + inner_h - h).max(inner_y);
        let max_x = (pr.x + pr.w - w).max(pr.x);
        for y in inner_y..max_y {
            for x in pr.x..max_x {
                if self.is_free(&Rect::new(x, y, w, h), Some(parent)) {
                    return Some((x, y));
                }
            }
        }
        None
    }

    /// Следующая свободная ячейка решётки автоподстановки (для объектов без явных координат).
    pub fn next_auto_slot(&mut self, w: i32, h: i32) -> (i32, i32) {
        loop {
            let s = self.auto_slot;
            self.auto_slot += 1;
            let gx = (s % AUTO_COLUMNS) as i32 * AUTO_PITCH_X;
            let gy = (s / AUTO_COLUMNS) as i32 * AUTO_PITCH_Y;
            if self.is_free(&Rect::new(gx, gy, w, h), None) {
                return (gx, gy);
            }
        }
    }

    // ---------- порты ----------
    /// Точка выхода k-го выходного показателя (правая граница карточки), в квантах.
    pub fn out_port(&self, o: ObjectIx, k: usize) -> Point {
        let r = self.rect[o.idx()];
        Point {
            x: (r.x + r.w) * Q,
            y: (r.y + 1 + k as i32) * Q + Q / 2,
        }
    }

    /// Точка входа k-го запроса (левая граница карточки), в квантах.
    pub fn request_port(&self, o: ObjectIx, k: usize) -> Point {
        let r = self.rect[o.idx()];
        Point {
            x: r.x * Q,
            y: (r.y + 1 + k as i32) * Q + Q / 2,
        }
    }

    // ---------- очередь перемаршрутизации ----------
    pub fn queue_route(&mut self, e: EdgeIx) {
        if !self.route_queued[e.idx()] {
            self.route_queued[e.idx()] = true;
            self.route_queue.push(e.0);
        }
    }

    pub(crate) fn take_route_queue(&mut self) -> Vec<u32> {
        let q = std::mem::take(&mut self.route_queue);
        for &e in &q {
            self.route_queued[e as usize] = false;
        }
        q
    }
}

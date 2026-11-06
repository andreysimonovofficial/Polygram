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
        for by in by0..=by1 {
            for bx in bx0..=bx1 {
                self.buckets.entry((bx, by)).or_default().push(id);
            }
        }
    }

    fn remove(&mut self, id: u32, r: &Rect) {
        let (bx0, by0, bx1, by1) = bucket_range(r);
        for by in by0..=by1 {
            for bx in bx0..=bx1 {
                if let Some(v) = self.buckets.get_mut(&(bx, by)) {
                    if let Some(p) = v.iter().position(|&x| x == id) {
                        v.swap_remove(p);
                    }
                    if v.is_empty() {
                        self.buckets.remove(&(bx, by));
                    }
                }
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
    pub fn is_free(&self, r: &Rect, ignore: Option<ObjectIx>) -> bool {
        let (bx0, by0, bx1, by1) = bucket_range(r);
        for by in by0..=by1 {
            for bx in bx0..=bx1 {
                if let Some(v) = self.grid.buckets.get(&(bx, by)) {
                    for &id in v {
                        if Some(ObjectIx(id)) == ignore {
                            continue;
                        }
                        if self.rect[id as usize].overlaps(r) {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }

    /// Номера объектов, чьи карточки пересекают прямоугольник (без повторов, по возрастанию).
    pub fn objects_overlapping(&self, r: &Rect, ignore: Option<ObjectIx>, out: &mut Vec<u32>) {
        out.clear();
        let (bx0, by0, bx1, by1) = bucket_range(r);
        for by in by0..=by1 {
            for bx in bx0..=bx1 {
                if let Some(v) = self.grid.buckets.get(&(bx, by)) {
                    for &id in v {
                        if Some(ObjectIx(id)) != ignore && self.rect[id as usize].overlaps(r) {
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
